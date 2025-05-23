use error_chain::bail;
use log::error;

use flowcore::errors::Result;
use flowcore::model::runtime_function::RuntimeFunction;

use crate::block::Block;
use crate::run_state::{RunState, State};

fn runtime_error(state: &RunState, job_id: usize, message: &str, file: &str, line: u32) -> Result<()> {
    let msg = format!("Job #{job_id}: Runtime error: at file: {file}, line: {line}\n
                        {message}\nJob #{job_id}: Error State -\n{state}");
    error!("{msg}");
    bail!(msg);
}

fn ready_check(state: &RunState, job_id: usize, function: &RuntimeFunction) -> Result<()> {
    if !state.get_busy_flows().contains_key(&function.get_flow_id()) {
        return runtime_error(
            state,
            job_id,
            &format!(
                "Function #{} is Ready, but Flow #{} is not busy",
                function.id(),
                function.get_flow_id()
            ),
            file!(),
            line!(),
        );
    }

    if !(state.get_function_states(function.id()).contains(&State::Ready) ||
        state.get_function_states(function.id()).contains(&State::Running)) {
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
    if state.get_running().contains_key(&job_id) && !state.get_busy_flows().contains_key(&function.get_flow_id()) {
        return runtime_error(
            state,
            job_id,
            &format!(
                "Function #{} is Running, but Flow #{} is not busy",
                function.id(),
                function.get_flow_id()
            ),
            file!(),
            line!(),
        );
    }

    Ok(())
}

fn blocked_check(state: &RunState, job_id: usize, function: &RuntimeFunction) -> Result<()> {
    if state.get_output_blockers(function.id()).is_empty() {
        return runtime_error(
            state,
            job_id,
            &format!(
                "Function #{} is in Blocked state, but no block for it exists",
                function.id()
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
fn completed_check(state: &RunState, job_id: usize, function: &RuntimeFunction,
                   function_states: &Vec<State>) -> Result<()> {
    if !(function_states.contains(&State::Completed) && function_states.len() == 1)
    {
        return runtime_error(
            state,
            job_id,
            &format!(
                "Function #{} has Completed, but states are: {:?}",
                function.id(), function_states
            ),
            file!(),
            line!(),
        );
    }

    Ok(())
}

// function should not be blocked on itself
fn self_block_check(state: &RunState, job_id: usize, block: &Block) -> Result<()> {
    if block.blocked_function_id == block.blocking_function_id {
        return runtime_error(
            state,
            job_id,
            &format!(
                "Block {block} has same Function id as blocked and blocking"),
            file!(),
            line!(),
        );
    }

    Ok(())
}

/*
fn destination_block_state_check(state: &RunState, job_id: usize, block: &Block,
                                 functions: &[RuntimeFunction]) -> Result<()> {
    // For each block on a destination function, then either that input should be full or
    // the function should be running in parallel with the one that just completed,
    // or it's flow should be busy and there should be a pending unblock on it
    if let Some(function) = functions.get(block.blocking_function_id) {
        if !(function.input_count(block.blocking_io_number) > 0
            || (state.get_busy_flows().contains_key(&block.blocking_flow_id)
            && state.get_flow_blocks().contains_key(&block.blocking_flow_id)))
        {
            return runtime_error(
                state,
                job_id,
                &format!("Block {block} exists for function #{}, but Function #{}:{} input is not full",
                         block.blocking_function_id, block.blocking_function_id, block.blocking_io_number),
                file!(), line!());
        }
    }

    Ok(())
}
 */

// Check pending unblock invariants
fn pending_unblock_checks(state: &RunState, job_id: usize) -> Result<()> {
    for pending_unblock_flow_id in state.get_flow_blocks().keys() {
        // flow it's in must be busy
        if !state.get_busy_flows().contains_key(pending_unblock_flow_id) {
            return runtime_error(
                state,
                job_id,
                &format!(
                    "Pending Unblock exists for Flow #{pending_unblock_flow_id}, but it is not busy"),
                file!(),
                line!(),
            );
        }
    }

    Ok(())
}

// Check block invariants
fn block_checks(state: &RunState, job_id: usize) -> Result<()> {
//    let functions = state.get_functions();
    for block in state.get_blocks() {
        self_block_check(state, job_id, block)?;
        //destination_block_state_check(state, job_id, block, functions)?;
    }

    Ok(())
}

fn function_state_checks(state: &RunState, job_id: usize) -> Result<()> {
    for function in state.get_functions() {
        running_check(state, job_id, function)?;

        let function_states = &state.get_function_states(function.id());
        for function_state in function_states {
            match function_state {
                State::Ready => ready_check(state, job_id, function)?,
                State::Blocked => blocked_check(state, job_id, function)?,
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
    function_state_checks(state, job_id)?;
    block_checks(state, job_id)?;
    pending_unblock_checks(state, job_id)
    //flow_checks(state, job_id)
}

#[cfg(test)]
mod test {
    #[cfg(feature = "debugger")]
    use serde_json::Value;

    #[cfg(feature = "debugger")]
    use flowcore::errors::Result;
    use flowcore::model::flow_manifest::FlowManifest;
    #[cfg(feature = "debugger")]
    use flowcore::model::input::Input;
    use flowcore::model::metadata::MetaData;
    #[cfg(feature = "debugger")]
    use flowcore::model::output_connection::OutputConnection;
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;

    #[cfg(feature = "debugger")]
    use crate::block::Block;
    use crate::checks::completed_check;
    #[cfg(feature = "debugger")]
    use crate::debug_command::DebugCommand;
    #[cfg(feature = "debugger")]
    use crate::debugger::Debugger;
    #[cfg(feature = "debugger")]
    use crate::debugger_handler::DebuggerHandler;
    #[cfg(feature = "debugger")]
    use crate::job::Job;
    use crate::run_state::{RunState, State};

    use super::blocked_check;
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
            #[cfg(feature = "debugger")] "test",
            #[cfg(feature = "debugger")] "/test",
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
        state.create_jobs_or_block(0, 0).expect("Couldn't get next job");

        // this ready_check() should pass
        ready_check(&state, 0, state.get_function(0)
            .ok_or("No function").expect("No function")).expect("Should pass");
    }

    #[test]
    fn test_ready_fails() {
        let function = test_function(0, 0);
        let state = test_state(vec![function]);

        // Do not mark flow_id as busy - if a function is ready, the flow containing it should
        // be marked as busy, so this ready_check() should fail

        assert!(ready_check(&state, 0, state.get_function(0)
            .ok_or("No function").expect("No function")).is_err());
    }

    #[test]
    fn test_running_passes() {
        let function = test_function(0, 0);
        let mut state = test_state(vec![function]);

        // mark flow_id as busy - to pass the running check a running function's flow_id
        // should be in the list of busy flows
        state.create_jobs_or_block(0, 0).expect("Couldn't get next job");

        // this running check should fail
        running_check(&state, 0, state.get_function(0)
            .ok_or("No function").expect("No function")).expect("Should pass");
    }

    #[cfg(feature = "debugger")]
    struct DummyServer;

    #[cfg(feature = "debugger")]
    impl DebuggerHandler for DummyServer {
        fn start(&mut self) {}
        fn job_breakpoint(&mut self, _job: &Job, _function: &RuntimeFunction, _states: Vec<State>) {}
        fn block_breakpoint(&mut self, _block: &Block) {}
        fn flow_unblock_breakpoint(&mut self, _flow_id: usize) {}
        fn send_breakpoint(&mut self, _: &str, _source_process_id: usize, _output_route: &str, _value: &Value,
                           _destination_id: usize, _destination_name:&str, _input_name: &str, _input_number: usize) {}
        fn job_error(&mut self, _job: &Job) {}
        fn job_completed(&mut self, _job: &Job) {}
        fn blocks(&mut self, _blocks: Vec<Block>) {}
        fn outputs(&mut self, _output: Vec<OutputConnection>) {}
        fn input(&mut self, _input: Input) {}
        fn function_list(&mut self, _functions: &[RuntimeFunction]) {}
        fn function_states(&mut self, _function: RuntimeFunction, _function_states: Vec<State>) {}
        fn run_state(&mut self, _run_state: &RunState) {}
        fn message(&mut self, _message: String) {}
        fn panic(&mut self, _state: &RunState, _error_message: String) {}
        fn debugger_exiting(&mut self) {}
        fn debugger_resetting(&mut self) {}
        fn debugger_error(&mut self, _error: String) {}
        fn execution_starting(&mut self) {}
        fn execution_ended(&mut self) {}
        fn get_command(&mut self, _state: &RunState) -> Result<DebugCommand> {
            unimplemented!();
        }
    }

    #[cfg(feature = "debugger")]
    fn dummy_debugger(server: &mut dyn DebuggerHandler) -> Debugger {
        Debugger::new(server)
    }

    #[test]
    fn test_blocked_passes() {
        let function = test_function(0, 0);
        let mut state = test_state(vec![function]);

        #[cfg(feature = "debugger")]
            let mut server = DummyServer{};

        #[cfg(feature = "debugger")]
            let mut debugger = dummy_debugger(&mut server);

        // mark function #0 as blocked on an imaginary function #1
        let _ = state.create_block(1,
            1,
            0,
            0, // <-- here
            0,
            #[cfg(feature = "debugger")] &mut debugger);

        // this blocked check should pass
        blocked_check(&state, 0, state.get_function(0)
            .ok_or("No function").expect("No function")).expect("Should pass");
    }

    #[test]
    fn test_blocked_fails() {
        let function = test_function(0, 0);
        let state = test_state(vec![function]);

        // Do NOT mark function #0 as blocked

        // this blocked check should fail
        assert!(blocked_check(&state, 0, state.get_function(0)
            .ok_or("No function").expect("No function")).is_err());
    }

    #[test]
    fn test_completed_passes() {
        let function = test_function(0, 0);
        let mut state = test_state(vec![function]);

        // Mark function #0 as completed
        state.mark_as_completed(0);
        let functions_states = vec![State::Completed];

        // this completed check should pass
        completed_check(&state, 0, state.get_function(0)
            .ok_or("No function").expect("No function"), &functions_states)
            .expect("Should pass");
    }

    #[test]
    fn test_completed_fails() {
        let function = test_function(0, 0);
        let state = test_state(vec![function]);

        // Do NOT mark function #0 as completed, use Ready state
        let functions_states = vec![State::Ready];

        // this completed check should fail
        assert!(completed_check(&state, 0,
                                state.get_function(0)
                                    .ok_or("No function").expect("No function"), &functions_states)
            .is_err());
    }
}