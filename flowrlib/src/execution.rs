use runnable::Runnable;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;

/*
    inputs_satisfied:
    A list of runnables who's inputs are satisfied.

    blocked:
    A list of tuples of runnable ids where first runnable_id is where data is trying
    to be sent to, and the second runnable_id is the runnable trying to send to it.
    Vec<(runnable_to_send_to, runnable_that_is_blocked_on_output)>

    Note that a runnable maybe blocking multiple others trying to send to it.
    Those others maybe blocked trying to send to multiple.
    So, when a runnable is run, we remove all entries that depend on it.
*/
struct RunList {
    runnables: Vec<Arc<Mutex<Runnable>>>,
    inputs_satisfied: HashSet<usize>,
    blocking: Vec<(usize, usize)>,
    ready: Vec<usize>
}

impl RunList {
    fn new() -> Self {
        RunList {
            runnables: Vec::<Arc<Mutex<Runnable>>>::new(),
            inputs_satisfied: HashSet::<usize>::new(),
            blocking: Vec::<(usize, usize)>::new(),
            ready: Vec::<usize>::new(),
        }
    }

    // Get a runnable from the runnable ID
    fn get(&self, id: usize) -> Arc<Mutex<Runnable>> {
        self.runnables[id].clone()
    }

    // save the fact that a particular Runnable's inputs are now satisfied
    fn inputs_ready(&mut self, id: usize) {
        info!("Runnable #{}'s inputs are all ready", id);

        if self.is_blocked(id) {
            self.inputs_satisfied.insert(id);
        } else {
            info!("Marking #{} as ready", id);
            self.ready.push(id);
        }
    }

    // Return the next runnable at the head of the ready list if there is one
    fn next(&mut self) -> Option<Arc<Mutex<Runnable>>> {
        if self.ready.len() == 0 {
            return None;
        }

        info!("Ready list: {:?}", self.ready);

        // get the ID of the next runnable to be run
        let id = self.ready.remove(0);
        Some(self.runnables[id].clone())
    }

    // Save the fact that the runnable 'blocked_id' is blocked on it's output by 'blocking_id'
    fn blocked_by(&mut self, blocking_id: usize, blocked_id: usize) {
        info!("Runnable #{} is blocking runnable #{}", &blocking_id, &blocked_id);
        self.blocking.push((blocking_id, blocked_id));
    }

    // unblock all runnables that were blocked trying to send to destination_id by removing all entries
    // in the list where the first value (blocking_id) matches the destination_id
    fn unblock_by(&mut self, destination_id: usize) {
        info!("Unblocking runnables blocked on #{}", &destination_id);
        for &(blocking_id, blocked_id) in &self.blocking {
            if blocking_id == destination_id {
                if self.inputs_satisfied.remove(&blocked_id) {
                    self.ready.push(blocked_id);
                }
            }
        }

        self.blocking.retain(|&(blocking_id, _blocked_id)| blocking_id != destination_id);
    }

    // See if there is any tuple in the vector where the second (blocked_id) is the one we're after
    fn is_blocked(&self, id: usize) -> bool {
        for &(_blocking_id, blocked_id) in &self.blocking {
            if blocked_id == id {
                return true;
            }
        }
        false
    }
}

/*
    This function is responsible for initializing all runnables. On initialization each one returns
    a boolean to indicate if they are now able to be run - if so it is placed in the ready queue
    which is returned.
*/
fn init(runnables: Vec<Arc<Mutex<Runnable>>>) -> RunList {
    let mut run_list = RunList::new();

    // TODO maybe a corner case where one value is outputting to another and
    // should be put on the blocked list even at the very start???

    info!("Initializing runnables");
    for runnable_arc in &runnables {
        let mut runnable = runnable_arc.lock().unwrap();
        info!("Initializing runnable #{}", &runnable.id());
        if runnable.init() {
            run_list.inputs_ready(runnable.id());
        }
    }

    run_list.runnables = runnables;
    run_list
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
/// use std::sync::{Arc, Mutex};
/// use flowrlib::runnable::Runnable;
/// use flowrlib::execution::execute;
///
/// let runnables = Vec::<Arc<Mutex<Runnable>>>::new();
///
/// execute(runnables);
/// ```
pub fn execute(runnables: Vec<Arc<Mutex<Runnable>>>) -> ! {
    let mut run_list = init(runnables);

    info!("Starting execution loop");
    while let Some(runnable_arc) = run_list.next() {
        let mut runnable = runnable_arc.lock().unwrap();
        info!("Running runnable #{}", runnable.id());
        let output = runnable.run();

        // If other runnables were blocked trying to send to this one - we can now unblock them
        run_list.unblock_by(runnable.id());

        for (destination_id, io_number) in runnable.output_destinations() {
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

    exit(0);
}