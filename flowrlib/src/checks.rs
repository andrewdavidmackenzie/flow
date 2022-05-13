use log::error;

use crate::run_state::{RunState, State};

#[cfg(debug_assertions)]
fn runtime_error(state: &RunState, job_id: usize, message: &str, file: &str, line: u32) {
    error!(
            "Job #{}: Runtime error: at file: {}, line: {}\n\t\t{}",
            job_id, file, line, message
        );
    error!("Job #{}: Error State - {}", job_id, state);
}

/// Check a number of "invariants" i.e. unbreakable rules about the state.
/// If one is found to be broken, report a runtime error explaining it, which may
/// trigger entry into the debugger.
#[cfg(debug_assertions)]
pub(crate) fn check_invariants(state: &RunState, job_id: usize) {
    let functions = state.get_functions();
    for function in functions {
        let function_states = &state.get_function_states(function.id());
        for function_state in function_states {
            match function_state {
                State::Ready => {
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
                }
                State::Running => {
                    if !state.get_busy_flows().contains_key(&function.get_flow_id()) {
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
                }
                State::Blocked => {
                    if !state.blocked_sending(function.id()) {
                        return runtime_error(
                            state,
                            job_id,
                            &format!(
                                "Function #{} is in Blocked state, but no block exists",
                                function.id()
                            ),
                            file!(),
                            line!(),
                        );
                    }
                }
                State::Waiting => {},
                State::Completed => {
                    // If completed, should not be in any of the other states
                    if function_states.len() > 1
                    {
                        return runtime_error(
                            state,
                            job_id,
                            &format!(
                                "Function #{} has Completed, but also appears as Ready or Blocked or Running",
                                function.id(),
                            ),
                            file!(),
                            line!(),
                        );
                    }
                },
            }
        }

        // State::Running is because functions with initializers auto-refill
        // So they will show as inputs full, but not Ready or Blocked
        if (!function.inputs().is_empty())
            && function.can_produce_output()
            && !(function_states.contains(&State::Ready)
                || function_states.contains(&State::Blocked)
                || function_states.contains(&State::Running))
        {
            return runtime_error(
                            state,
                            job_id,
                    &format!(
                                "Function #{} inputs have data, but it is not Ready or Blocked or Running",
                                function.id()),
                        file!(),
                        line!(),
            );
        }
    }

    // Check block invariants
    for block in state.get_blocks() {
        // function should not be blocked on itself
        if block.blocked_function_id == block.blocking_function_id {
            return runtime_error(
                            state,
                            job_id,
                    &format!(
                                "Block {} has same Function id as blocked and blocking",
                                block),
                        file!(),
                        line!(),
            );
        }

        // For each block on a destination function, then either that input should be full or
        // the function should be running in parallel with the one that just completed
        // or it's flow should be busy and there should be a pending unblock on it
        if let Some(function) = functions.get(block.blocking_function_id) {
            if !(function.input_count(block.blocking_io_number) > 0
                || (state.get_busy_flows().contains_key(&block.blocking_flow_id)
                && state.get_pending_unblocks().contains_key(&block.blocking_flow_id)))
            {
                return runtime_error(
                    state,
                    job_id,
                    &format!("Block {} exists for function #{}, but Function #{}:{} input is not full",
                             block, block.blocking_function_id, block.blocking_function_id, block.blocking_io_number),
                    file!(), line!());
            }
        }
    }

    // Check pending unblock invariants
    for pending_unblock_flow_id in state.get_pending_unblocks().keys() {
        // flow it's in must be busy
        if !state.get_busy_flows().contains_key(pending_unblock_flow_id) {
            return runtime_error(
                            state,
                            job_id,
                    &format!(
                                "Pending Unblock exists for Flow #{}, but it is not busy",
                                pending_unblock_flow_id),
                    file!(),
                    line!(),
            );
        }
    }

    // Check busy flow invariants
    for (flow_id, function_id) in state.get_busy_flows().iter() {
        if !state.function_states_includes(*function_id, State::Ready) &&
            !state.function_states_includes(*function_id, State::Running) {
            return runtime_error(
                            state,
                            job_id,
                    &format!("Busy flow entry exists for Function #{} in Flow #{} but it's not Ready nor Running",
                                                       function_id, flow_id),
                    file!(), line!());
        }
    }
}