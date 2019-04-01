use std::panic;
use coordinator::Job;
use coordinator::Output;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;

pub fn looper(name: String, job_rx: Receiver<Job>, output_tx: Sender<Output>) {
    // TODO spawn thread with unique name
    let builder = thread::Builder::new().name(name);
    builder.spawn(move || {
        set_panic_hook();

        loop {
            match job_rx.recv() {
                Ok(job) => {
                    execute(job, &output_tx);
                }
                _ => break
            }
        }
    }).unwrap();
}

pub fn execute(dispatch: Job, output_tx: &Sender<Output>) {
    // Run the implementation with the input values and catch the execution result
    let (result, error) = match panic::catch_unwind(|| {
        dispatch.implementation.run(dispatch.input_values.clone())
    }) {
        Ok(result) => (result, None),
        Err(_   ) => ((None, false), Some("Execution panicked".into())),
    };

    let output = Output {
        function_id: dispatch.function_id,
        input_values: dispatch.input_values,
        result,
        destinations: dispatch.destinations,
        error
    };

    let _sent = output_tx.send(output);
}

/*
    Replace the standard panic hook with one that just outputs the file and line of any process's
    runtime panic.
*/
fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(location) = panic_info.location() {
            error!("panic occurred in file '{}' at line {}", location.file(), location.line());
        }
    }));
}