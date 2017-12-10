use runnable::Runnable;
use std::process::exit;
use std::sync::{Arc, Mutex};

/*
    This function is responsible for initializing all runnables. On initialization each one returns
    a boolean to indicate if they are now able to be run - if so it is placed in the ready queue
    which is returned.
*/
fn init(runnables: &Vec<Arc<Mutex<Runnable>>>) -> Vec<Arc<Mutex<Runnable>>> {
    let mut ready = Vec::<Arc<Mutex<Runnable>>>::new();

    info!("Initializing values");
    for runnable_arc_ref in runnables {
        let runnable_arc = runnable_arc_ref.clone();
        let mut runnable_mut = runnable_arc.lock().unwrap();
        if runnable_mut.init() {
            ready.push(runnable_arc.clone());
        }
    }

    ready
}

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
/// use flowrlib::runnable::Runnable;
/// use flowrlib::execution::execute;
///
/// let runnables = Vec::<Box<Runnable>>::new();
///
/// execute(runnables);
/// ```
pub fn execute(runnables: Vec<Arc<Mutex<Runnable>>>) -> ! {
    let mut ready = init(&runnables);

    info!("Starting execution loop");
    loop {
        let runnable_arc = ready.remove(0).clone();
        let mut runnable_mut = runnable_arc.lock().unwrap();
        let output = runnable_mut.run();
//        info!("Output = '{}'", output.unwrap());

        for (run_id, io_number) in runnable_mut.get_affected() {
            let affected_arc = runnables[run_id].clone();
            let mut affected = affected_arc.lock().unwrap();
            if affected.write_input(io_number, output.clone()){
                ready.push(affected_arc.clone());
            }
        }

        // See if that produced any changes in other runnables, such that they should be added to
        // the runnables list.

        if ready.is_empty() {
            exit(0);
        }
    }
}