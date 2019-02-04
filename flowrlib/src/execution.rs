use process::Process;
use runlist::RunList;
use std::panic;
use std::sync::{Arc, Mutex};
use log::LogLevel::Debug;

/// The generated code for a flow consists of values and functions formed into a list of Processs.
///
/// This list is built program start-up in `main` which then starts execution of the flow by calling
/// this `execute` method.
///
/// You should not have to write code to call `execute` yourself, it will be called from the
/// generated code in the `main` method.
///
/// On completion of the execution of the flow it will return and `main` will call `exit`
///
/// # Example
/// ```
/// use std::sync::{Arc, Mutex};
/// use flowrlib::process::Process;
/// use flowrlib::execution::execute;
/// use std::process::exit;
///
/// let mut processs = Vec::<Arc<Mutex<Process>>>::new();
///
/// execute(processs, false /* use_debugger */);
///
/// exit(0);
/// ```
pub fn execute(processs: Vec<Arc<Mutex<Process>>>, use_debugger: bool) {
    set_panic_hook();
    let mut run_list = init(processs);

    debug!("Starting execution loop");
    debug!("-----------------------------------------------------------------");
    if log_enabled!(Debug) {
        run_list.print_state();
    }

    if use_debugger {
        #[cfg(feature = "debugger")]
        run_list.debug();
    }

    while let Some(id) = run_list.next() {
        dispatch(&mut run_list, id);
        if log_enabled!(Debug) {
            run_list.print_state();
        }
    }
    debug!("Ended execution loop");

    run_list.end();
}

/*
    Given a process id, start running it
*/
fn dispatch(run_list: &mut RunList, id: usize) {
    let process_arc = run_list.get(id);
    let process: &mut Process = &mut *process_arc.lock().unwrap();
    debug!("Process #{} '{}' dispatched", id, process.name());

    let input_values = process.get_input_values();
    run_list.inputs_consumed(id);
    run_list.unblock_senders_to(id);
    debug!("\tProcess #{} '{}' running with inputs: {:?}", id, process.name(), input_values);

    let implementation = process.get_implementation();

    // when a process ends, it can express whether it can run again or not
    let (value, run_again) = implementation.run(input_values);

    if let Some(val) = value {
        debug!("\tProcess #{} '{}' completed, send output '{}'", id, process.name(), &val);
        run_list.process_output(process, val);
    } else {
        debug!("\tProcess #{} '{}' completed, no output", id, process.name());
    }
    // if it wants to run again and it can (inputs ready) then add back to the Can Run list
    if run_again && process.can_run() {
        run_list.can_run(process.id());
    }
}

/*
    Replace the standard panic hook with one that just outputs the file and line of any process's
    runtime panic.
*/
fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(location) = panic_info.location() {
            error!("panic occurred in file '{}' at line {}", location.file(), location.line());
        } else {
            error!("panic occurred but can't get location information...");
        }
    }));
    debug!("Panic hook set to catch panics in processs");
}

/*
    The ìnit' function is responsible for initializing all processs.
    The `init` method on each process is called, which returns a boolean to indicate that it's
    inputs are fulfilled - and this information is added to the RunList to control the readyness of
    the Process to be executed.

    Once all processs have been initialized, the list of processs is stored in the RunList
*/
fn init(processs: Vec<Arc<Mutex<Process>>>) -> RunList {
    let mut run_list = RunList::new();

    debug!("Initializing all processs");
    for process_arc in &processs {
        let mut process = process_arc.lock().unwrap();
        debug!("\tInitializing process #{} '{}'", &process.id(), process.name());
        if process.init() {
            run_list.can_run(process.id());
        }
    }

    run_list.set_processs(processs);

    run_list
}