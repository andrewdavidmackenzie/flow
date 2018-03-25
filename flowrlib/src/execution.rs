use runnable::Runnable;
use std::sync::{Arc, Mutex};
use runlist::RunList;
use serde_json::Value as JsonValue;

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
    let mut run_list = init(runnables);

    debug!("Starting execution loop");
    while let Some(id) = run_list.next() {
        let runnable_arc = run_list.get(id);
        let mut runnable = runnable_arc.lock().unwrap();
        debug!("Running runnable: #{} '{}'", id, runnable.name());
        let output = runnable.run();

        // TODO ADM figure out why this crashes fibonacci
    //    if output != JsonValue::Null {
            debug!("Processing output of runnable: #{} '{}'", id, runnable.name());
            run_list.process_output(&*runnable, output);
  //      }
    }
    debug!("Ended execution loop");

    run_list.end();
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
        debug!("Initializing runnable #{}", &runnable.id());
        if runnable.init() {
            run_list.inputs_ready(runnable.id());
        }
    }

    run_list.set_runnables(runnables);
    run_list
}