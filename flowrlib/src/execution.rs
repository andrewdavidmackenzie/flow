use runnable::Runnable;
use std::sync::{Arc, Mutex};
use runlist::RunList;
use std::panic;

/// The generated code for a flow consists of values and functions formed into a list of Runnables.
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
/// use flowrlib::runnable::Runnable;
/// use flowrlib::execution::execute;
/// use std::process::exit;
///
/// let mut runnables = Vec::<Arc<Mutex<Runnable>>>::new();
///
/// execute(runnables);
///
/// exit(0);
/// ```
pub fn execute(runnables: Vec<Arc<Mutex<Runnable>>>) {
    set_panic_hook();
    let mut run_list = init(runnables);

    debug!("Starting execution loop");
    debug!("-----------------------------------------------------------------");
    run_list.debug();
    while let Some(id) = run_list.next() {
        dispatch(&mut run_list, id);
        run_list.debug();
    }
    debug!("Ended execution loop");

    run_list.end();
}

/*
    Given a runnable id, start running it
*/
fn dispatch(run_list: &mut RunList, id: usize) {
    let runnable_arc = run_list.get(id);
    let runnable: &mut Runnable = &mut *runnable_arc.lock().unwrap();
    debug!("Runnable #{} '{}' dispatched", id, runnable.name());

    let inputs = runnable.get_inputs();
    run_list.inputs_consumed(id);
    run_list.unblock_senders_to(id);
    debug!("\tRunnable #{} '{}' running with inputs: {:?}", id, runnable.name(), inputs);

    let implementation = runnable.implementation();

    // if after all is said and done it can run again, then add to the end of the ready list
    let run_again = implementation.run(runnable, inputs, run_list);

    // if it wants to run again and it can (inputs ready) then add back to the Can Run list
    if run_again && runnable.can_run() {
        run_list.can_run(runnable.id());
    }
}

/*
    Replace the standard panic hook with one that just outputs the file and line of any runnable's
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
    debug!("Panic hook set to catch panics in runnables");
}

/*
    The ìnit' function is responsible for initializing all runnables.
    The `init` method on each runnable is called, which returns a boolean to indicate that it's
    inputs are fulfilled - and this information is added to the RunList to control the readyness of
    the Runnable to be executed.

    Once all runnables have been initialized, the list of runnables is stored in the RunList
*/
fn init(runnables: Vec<Arc<Mutex<Runnable>>>) -> RunList {
    let mut run_list = RunList::new();

    debug!("Initializing all runnables");
    for runnable_arc in &runnables {
        let mut runnable = runnable_arc.lock().unwrap();
        debug!("\tInitializing runnable #{} '{}'", &runnable.id(), runnable.name());
        if runnable.init() {
            run_list.can_run(runnable.id());
        }
    }

    run_list.set_runnables(runnables);
    run_list
}