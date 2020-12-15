use std::panic;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use log::error;
use log::trace;

use crate::errors::*;
use crate::run_state::Job;

/*
    Start a number of executor threads that all listen on the 'job_rx' channel for
    Jobs to execute and return the Outputs on the 'output_tx' channel
*/
pub fn start_executors(number_of_executors: usize,
                       job_rx: &Arc<Mutex<Receiver<Job>>>,
                       job_tx: &Sender<Job>) {
    for executor_number in 0..number_of_executors {
        create_executor(format!("Executor #{}", executor_number),
                        job_rx.clone(), job_tx.clone());
    }
}

/*
    Replace the standard panic hook with one that just outputs the file and line of any process's
    run-time panic.
*/
pub fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        /* Only available on 'nightly'
        if let Some(message) = panic_info.message() {
            error!("Message: {:?}", message);
        }
        */

        if let Some(location) = panic_info.location() {
            error!("Panic in file '{}' at line {}", location.file(), location.line());
        }
    }));
}

fn create_executor(name: String, job_rx: Arc<Mutex<Receiver<Job>>>, job_tx: Sender<Job>) {
    let builder = thread::Builder::new();
    let _ = builder.spawn(move || {
        set_panic_hook();

        loop {
            let _ = get_and_execute_job(&job_rx, &job_tx, &name);
        }
    });
}

fn get_and_execute_job(job_rx: &Arc<Mutex<Receiver<Job>>>,
                       job_tx: &Sender<Job>,
                       name: &str) -> Result<()> {
    let guard = job_rx.lock()
        .map_err(|e| format!("Error locking receiver to get job: '{}'", e))?;
    let job = guard.recv().map_err(|e| format!("Error receiving job for execution: '{}'", e))?;
    execute(job, job_tx, name)
}

fn execute(mut job: Job, job_tx: &Sender<Job>, name: &str) -> Result<()> {
    // Run the job and catch the execution result
    trace!("Job #{}:\tExecuting on '{}'", job.job_id, name);
    let result = job.implementation.run(&job.input_set);

    job.result = result;
    job_tx.send(job).chain_err(|| "Error sending job result back after execution")
}