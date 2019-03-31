use std::panic;
use coordinator::Job;
use coordinator::Output;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;

pub fn looper(job_rx: Receiver<Job>, output_tx: Sender<Output>) {
    thread::spawn(move || {
        set_panic_hook();

        loop {
            match job_rx.recv() {
                Ok(job) => {
                    debug!("Received dispatch over channel");
                    execute(job, &output_tx);
                }
                _ => break
            }
        }
    });
}

pub fn execute(dispatch: Job, output_tx: &Sender<Output>) {
    // Run the implementation with the input values and catch the execution result
    let result = dispatch.implementation.run(dispatch.input_values.clone());

    let output = Output {
        function_id: dispatch.function_id,
        input_values: dispatch.input_values,
        result,
        destinations: dispatch.destinations,
    };

    match output_tx.send(output) {
        Err(_) => error!("Error sending output on 'output_tx' channel"),
        _ => debug!("Returned Function Output over channel")
    };
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
            error!("panic occurred but can't get location information");
        }
    }));
    debug!("Panic hook set to catch panics in process execution");
}