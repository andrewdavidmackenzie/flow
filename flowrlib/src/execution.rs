use runnable::Runnable;
use std::process::exit;

/*
    This function is responsible for initializing all runnables. Each one returns a boolean to
    indicate if they are now able to be run - and if so it is placed in the ready queue which is
    returned.
*/
fn init(runnables: Vec<Box<Runnable>>) -> Vec<Box<Runnable>> {
    let mut ready = Vec::<Box<Runnable>>::new();

    info!("Initializing values");
    for mut runnable in runnables {
        if runnable.init() {
            ready.push(runnable);
        }/* else {
            blocked.push(runnable);
        }*/
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
pub fn execute(runnables: Vec<Box<Runnable>>) -> ! {
    let mut ready = init(runnables);

    info!("Starting execution loop");
    loop {
        for mut runnable in ready {
            runnable.run();

//            blocked.push(runnable);

            // for everything that is listening on the output of the function/value that was just
            // run... (if the function produces no output, then no one will be listening and null list
            // their status needs to be checked...
//        functions[0].implementation.run(&mut functions[0]);
        }

        ready = Vec::<Box<Runnable>>::new();

        if ready.is_empty() {
            exit(0);
        }
    }
}