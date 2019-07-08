use std::panic;
use crate::run_state::{Job, Output};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

/*
    Start a number of executor threads that all listen on the 'job_rx' channel for
    Jobs to execute and return the Outputs on the 'output_tx' channel
*/
pub fn start_executors(number_of_executors: usize,
                       job_rx: &Arc<Mutex<Receiver<Job>>>,
                       output_tx: &Sender<Output>) {
    for executor_number in 0..number_of_executors {
        create_executor(format!("Executor #{}", executor_number),
                        job_rx.clone(), output_tx.clone());
    }
}

pub fn get_and_execute_job(job_rx: &Arc<Mutex<Receiver<Job>>>, output_tx: &Sender<Output>) -> Result<(), String> {
    let job = job_rx.lock().unwrap().recv();
    match job {
        Ok(job) => execute(job, output_tx),
        Err(e) => Err(e.to_string())
    }
}

fn create_executor(name: String, job_rx: Arc<Mutex<Receiver<Job>>>, output_tx: Sender<Output>) {
    let builder = thread::Builder::new().name(name);
    builder.spawn(move || {
        set_panic_hook();

        loop {
            get_and_execute_job(&job_rx, &output_tx).unwrap();
        }
    }).unwrap();
}

fn execute(job: Job, output_tx: &Sender<Output>) -> Result<(), String> {
    // Run the implementation with the input values and catch the execution result
    let (result, error) = match panic::catch_unwind(|| {
        job.implementation.run(job.input_set.clone())
    }) {
        Ok(result) => (result, None),
        Err(_) => ((None, false), Some("Execution panicked".into())),
    };

    let output = Output {
        job_id: job.job_id,
        function_id: job.function_id,
        input_values: job.input_set,
        result,
        destinations: job.destinations,
        error,
    };

    output_tx.send(output).unwrap();

    Ok(())
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