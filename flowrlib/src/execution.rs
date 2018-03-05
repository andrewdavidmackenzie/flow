use runnable::Runnable;
use std::sync::{Arc, Mutex};
use runlist::RunList;

/// The generated code for a flow consists of values and functions. Once these lists have been
/// loaded at program start-up then start executing the program using the `execute` method.
/// You should not have to write code to use this method yourself, it will be called from the
/// generated code in the `main` method.
///
/// It is a divergent function that will never return. On completion of the execution of the flow
/// it will exit the process.
///
/// # Example
/// ```
/// use std::sync::{Arc, Mutex};
/// use flowrlib::runnable::Runnable;
/// use flowrlib::execution::execute;
/// use std::process::exit;
///
/// let runnables = Vec::<Arc<Mutex<Runnable>>>::new();
///
/// execute(runnables);
///
/// exit(0);
/// ```
pub fn execute(runnables: Vec<Arc<Mutex<Runnable>>>) {
    let mut run_list = init(runnables);

    debug!("Starting execution loop");
    while let Some(runnable_arc) = run_list.next() {
        let mut runnable = runnable_arc.lock().unwrap();
        debug!("Running runnable #{}", runnable.id());
        let output = runnable.run();

        // If other runnables were blocked trying to send to this one - we can now unblock them
        // as it has consumed it's inputs and they are free to be sent to again.
        run_list.unblock_by(runnable.id());

        for &(destination_id, io_number) in runnable.output_destinations() {
            let destination_arc = run_list.get(destination_id);
            let mut destination = destination_arc.lock().unwrap();
            info!("Sending output '{:?}' to ({}, {})", &output, &destination_id, &io_number);
            run_list.blocked_by(destination_id, runnable.id());
            destination.write_input(io_number, output.clone());
            if destination.inputs_satisfied() {
                run_list.inputs_ready(destination_id);
            }
        }
    }
}


/// The ìnit' function is responsible for initializing all runnables.
/// The ìnit`method on each runnable is called, which returns a boolean to indicate that it's
/// inputs are fulfilled - and this information is added to the RunList to control the readyness of
/// the Runnable to be executed
fn init(runnables: Vec<Arc<Mutex<Runnable>>>) -> RunList {
    let mut run_list = RunList::new();

    // TODO maybe a corner case where one value is outputting to another and
    // should be put on the blocked list even at the very start???

    debug!("Initializing runnables");
    for runnable_arc in &runnables {
        let mut runnable = runnable_arc.lock().unwrap();
        debug!("Initializing runnable #{}", &runnable.id());
        if runnable.init() {
            debug!("Runnable #{} inputs ready, added to run list", &runnable.id());
            run_list.inputs_ready(runnable.id());
        }
    }

    run_list.set_runnables(runnables);
    run_list
}