use error_chain::bail;
use log::error;

use flowcore::errors::Result;
use flowcore::model::runtime_function::RuntimeFunction;

use crate::run_state::{RunState, State};

fn runtime_error(
    state: &RunState,
    job_id: usize,
    message: &str,
    file: &str,
    line: u32,
) -> Result<()> {
    let msg = format!(
        "Job #{job_id}: Runtime error: at file: {file}, line: {line}\n
                        {message}\nJob #{job_id}: Error State -\n{state}"
    );
    error!("{msg}");
    bail!(msg);
}

fn ready_check(state: &RunState, job_id: usize, function: &RuntimeFunction) -> Result<()> {
    if !state
        .get_busy_count()
        .contains_key(&function.get_parent_id())
    {
        return runtime_error(
            state,
            job_id,
            &format!(
                "Function #{} is Ready, but Flow #{} is not busy",
                function.id(),
                function.get_parent_id()
            ),
            file!(),
            line!(),
        );
    }

    if !(state
        .get_function_states(function.id())
        .contains(&State::Ready)
        || state
            .get_function_states(function.id())
            .contains(&State::Running))
    {
        return runtime_error(
            state,
            job_id,
            &format!(
                "Flow is busy but Function #{} is not Ready or Running",
                function.id(),
            ),
            file!(),
            line!(),
        );
    }

    Ok(())
}

fn running_check(state: &RunState, job_id: usize, function: &RuntimeFunction) -> Result<()> {
    if state.get_running().contains_key(&job_id)
        && !state
            .get_busy_count()
            .contains_key(&function.get_parent_id())
    {
        return runtime_error(
            state,
            job_id,
            &format!(
                "Function #{} is Running, but Flow #{} is not busy",
                function.id(),
                function.get_parent_id()
            ),
            file!(),
            line!(),
        );
    }

    Ok(())
}

// Empty test for waiting state to have checks for all state
#[allow(clippy::unnecessary_wraps)]
fn waiting_check(_state: &RunState, _job_id: usize, _function: &RuntimeFunction) -> Result<()> {
    // TODO
    Ok(())
}

// If function has completed, its States should contain Completed and only Completed
fn completed_check(
    state: &RunState,
    job_id: usize,
    function: &RuntimeFunction,
    function_states: &Vec<State>,
) -> Result<()> {
    if !(function_states.contains(&State::Completed) && function_states.len() == 1) {
        return runtime_error(
            state,
            job_id,
            &format!(
                "Function #{} has Completed, but states are: {:?}",
                function.id(),
                function_states
            ),
            file!(),
            line!(),
        );
    }

    Ok(())
}

fn function_state_checks(state: &RunState, job_id: usize) -> Result<()> {
    for function in state.get_functions().values() {
        running_check(state, job_id, function)?;

        let function_states = &state.get_function_states(function.id());
        for function_state in function_states {
            match function_state {
                State::Ready => ready_check(state, job_id, function)?,
                State::Waiting => waiting_check(state, job_id, function)?,
                State::Completed => completed_check(state, job_id, function, function_states)?,
                State::Running => {}
            }
        }
    }

    Ok(())
}

// Check busy flow invariants
/*
fn flow_checks(state: &RunState, job_id: usize) -> Result<()> {
    for (flow_id, function_id) in state.get_busy_flows().iter() {
        let function_states = state.get_function_states(*function_id);
        if !function_states.contains(&State::Ready) && !state.get_running().contains_key(&job_id) {
            return runtime_error(
                state,
                job_id,
                &format!("Busy flow entry exists for Function #{function_id} in Flow #{flow_id} but it's not Ready nor Running"),
                file!(), line!());
        }
    }

    Ok(())
}
 */

/// Check a number of "invariants" i.e. unbreakable rules about the state.
/// If one is found to be broken, report a runtime error explaining it, which may
/// trigger entry into the debugger.
pub(crate) fn check_invariants(state: &RunState, job_id: usize) -> Result<()> {
    function_state_checks(state, job_id)
    //flow_checks(state, job_id)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use flowcore::model::flow_manifest::FlowManifest;
    use flowcore::model::metadata::MetaData;
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;

    use crate::checks::completed_check;
    use crate::run_state::{RunState, State};

    use super::ready_check;
    use super::running_check;

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".into()],
        }
    }

    fn test_manifest(functions: Vec<RuntimeFunction>) -> FlowManifest {
        let mut manifest = FlowManifest::new(test_meta_data());
        for function in functions {
            manifest.add_function(function);
        }
        manifest
    }

    fn test_submission(functions: Vec<RuntimeFunction>) -> Submission {
        Submission::new(
            test_manifest(functions),
            None,
            None,
            #[cfg(feature = "debugger")]
            true,
        )
    }

    fn test_state(functions: Vec<RuntimeFunction>) -> RunState {
        RunState::new(test_submission(functions))
    }

    fn test_function(function_id: usize, flow_id: usize) -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "test",
            #[cfg(feature = "debugger")]
            "/test",
            "file://fake/test",
            vec![],
            function_id,
            flow_id,
            &[],
            true,
        )
    }

    #[test]
    fn test_ready_passes() {
        let function = test_function(0, 0);
        let mut state = test_state(vec![function]);

        // Mark the flow the function is in as busy via ready
        state.create_jobs(0, 0).expect("Couldn't create jobs");

        // this ready_check() should pass
        ready_check(
            &state,
            0,
            state
                .get_function(0)
                .ok_or("No function")
                .expect("No function"),
        )
        .expect("Should pass");
    }

    #[test]
    fn test_ready_fails() {
        let function = test_function(0, 0);
        let state = test_state(vec![function]);

        // Do not mark flow_id as busy - if a function is ready, the flow containing it should
        // be marked as busy, so this ready_check() should fail

        assert!(ready_check(
            &state,
            0,
            state
                .get_function(0)
                .ok_or("No function")
                .expect("No function")
        )
        .is_err());
    }

    #[test]
    fn test_running_passes() {
        let function = test_function(0, 0);
        let mut state = test_state(vec![function]);

        // mark flow_id as busy - to pass the running check a running function's flow_id
        // should be in the list of busy flows
        state.create_jobs(0, 0).expect("Couldn't create jobs");

        // this running check should pass
        running_check(
            &state,
            0,
            state
                .get_function(0)
                .ok_or("No function")
                .expect("No function"),
        )
        .expect("Should pass");
    }

    #[test]
    fn test_completed_passes() {
        let function = test_function(0, 0);
        let mut state = test_state(vec![function]);

        // Mark function #0 as completed
        state.mark_as_completed(0);
        let functions_states = vec![State::Completed];

        // this completed check should pass
        completed_check(
            &state,
            0,
            state
                .get_function(0)
                .ok_or("No function")
                .expect("No function"),
            &functions_states,
        )
        .expect("Should pass");
    }

    #[test]
    fn test_completed_fails() {
        let function = test_function(0, 0);
        let state = test_state(vec![function]);

        // Do NOT mark function #0 as completed, use Ready state
        let functions_states = vec![State::Ready];

        // this completed check should fail
        assert!(completed_check(
            &state,
            0,
            state
                .get_function(0)
                .ok_or("No function")
                .expect("No function"),
            &functions_states
        )
        .is_err());
    }
}
