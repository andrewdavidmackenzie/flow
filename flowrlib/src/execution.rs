use std::panic;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use log::{error, info};
use log::trace;

use flowcore::errors::*;

use crate::job::Job;

pub struct Executor {
    /// A channel used to send Jobs out for execution
    job_tx: Sender<Job>,
    /// A channel used to receive Jobs back after execution (now including the job's output)
    job_rx: Receiver<Job>,
    /// The timeout for waiting for results back from jobs being executed
    job_timeout: Option<Duration>,
}

/// Struct that takes care of execution of jobs, sending jobs for execution and receiving results
impl Executor {
    pub fn new(number_of_executors: usize, job_timeout: Option<Duration>,) -> Self {
        let (job_tx, job_rx) = mpsc::channel();
        let (output_tx, output_rx) = mpsc::channel();

        info!("Starting {} local executor threads", number_of_executors);
        let shared_job_receiver = Arc::new(Mutex::new(job_rx));
        start_executors(number_of_executors, &shared_job_receiver, &output_tx);

        Executor {
            job_tx,
            job_rx: output_rx,
            job_timeout
        }
    }

    /// Set the timeout to use when waiting for job results after execution
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.job_timeout = timeout;
    }

    /// Wait for, then return the next Job with results returned from executors
    pub fn get_next_result(&mut self) -> Result<Job> {
        match self.job_timeout {
            Some(t) => self.job_rx.recv_timeout(t)
                .chain_err(|| "Timeout while waiting for Job result"),
            None => self.job_rx.recv()
                .chain_err(|| "Error while trying to receive Job results")
        }
    }

    pub fn send_job_for_execution(&mut self, job: &Job) -> Result<()> {
        self.job_tx
            .send(job.clone())
            .chain_err(|| "Sending of job for execution failed")?;

        trace!(
            "Job #{}: Sent for Execution of Function #{}",
            job.job_id,
            job.function_id
        );

        Ok(())
    }
}

// Start a number of executor threads that all listen on the 'job_rx' channel for
// Jobs to execute and return the Outputs on the 'output_tx' channel
fn start_executors(
    number_of_executors: usize,
    job_rx: &Arc<Mutex<Receiver<Job>>>,
    job_tx: &Sender<Job>,
) {
    for executor_number in 0..number_of_executors {
        create_executor(
            format!("Executor #{}", executor_number),
            job_rx.clone(),
            job_tx.clone(),
        ); // clone of Arcs and Sender OK
    }
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

// Replace the standard panic hook with one that just outputs the file and line of any process's
// run-time panic.
fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        /* Only available on 'nightly'
        if let Some(message) = panic_info.message() {
            error!("Message: {:?}", message);
        }
        */

        if let Some(location) = panic_info.location() {
            error!(
                "Panic in file '{}' at line {}",
                location.file(),
                location.line()
            );
        }
    }));
}

fn get_and_execute_job(
    job_rx: &Arc<Mutex<Receiver<Job>>>,
    job_tx: &Sender<Job>,
    name: &str,
) -> Result<()> {
    let guard = job_rx
        .lock()
        .map_err(|e| format!("Error locking receiver to get job: '{}'", e))?;
    let job = guard
        .recv()
        .map_err(|e| format!("Error receiving job for execution: '{}'", e))?;
    execute(job, job_tx, name)
}

fn execute(mut job: Job, job_tx: &Sender<Job>, name: &str) -> Result<()> {
    trace!("Job #{}: Started  executing on '{name}'", job.job_id);
    job.result = job.implementation.run(&job.input_set);
    trace!("Job #{}: Finished executing on '{name}'", job.job_id);
    job_tx
        .send(job)
        .chain_err(|| "Error sending job result back after execution")
}
