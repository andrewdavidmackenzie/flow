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
                _ => {},
            }
        }

        // State::Running is because functions with initializers auto-refill
        // So they will show as inputs full, but not Ready or Blocked
        if (!function.inputs().is_empty())
            && function.is_runnable()
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
}