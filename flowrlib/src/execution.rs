use std::panic;
use run_state::{Job, Output};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

/*
    Start a number of executor threads that all listen on the 'job_rx' channel for
    Jobs to execute and return the Outputs on the 'output_tx' channel
*/
pub fn start_executors(number_of_executors: usize,
                       job_rx: Receiver<Job>,
                       output_tx: Sender<Output>) {
    let shared_job_receiver = Arc::new(Mutex::new(job_rx));
    for executor_number in 0..number_of_executors {
        create_executor(format!("Executor #{}", executor_number),
                        Arc::clone(&shared_job_receiver), output_tx.clone());
    }
}

fn create_executor(name: String, job_rx: Arc<Mutex<Receiver<Job>>>, output_tx: Sender<Output>) {
    let builder = thread::Builder::new().name(name);
    builder.spawn(move || {
        set_panic_hook();

        loop {
            let job = job_rx.lock().unwrap().recv();
            match job {
                Ok(job) => {
                    execute(job, &output_tx);
                }
                _ => break
            }
        }
    }).unwrap();
}

fn execute(job: Job, output_tx: &Sender<Output>) {
    // Run the implementation with the input values and catch the execution result
    let (result, error) = match panic::catch_unwind(|| {
        job.implementation.run(job.input_values.clone())
    }) {
        Ok(result) => (result, None),
        Err(_) => ((None, false), Some("Execution panicked".into())),
    };

    let output = Output {
        function_id: job.function_id,
        input_values: job.input_values,
        result,
        destinations: job.destinations,
        error,
    };

    output_tx.send(output).unwrap();
}

/*
    Replace the standard panic hook with one that just outputs the file and line of any process's
    runtime panic.
*/
pub fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(location) = panic_info.location() {
            error!("panic occurred in file '{}' at line {}", location.file(), location.line());
        }
    }));
}