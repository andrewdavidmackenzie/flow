use std::io;
use std::io::Write;

use log::error;

use flowrlib::client_server::DebugClientConnection;
use flowrlib::debug::{Event, Event::{*}, Param, Response, Response::{*}};

use crate::errors::*;

const HELP_STRING: &str = "Debugger commands:
'b' | 'breakpoint' {spec}    - Set a breakpoint on a function (by id), an output or an input using spec:
                                - function_id
                                - source_id/output_route ('source_id/' for default output route)
                                - destination_id:input_number
                                - blocked_process_id->blocking_process_id
ENTER | 'c' | 'continue'     - Continue execution until next breakpoint
'd' | 'delete' {spec} or '*' - Delete the breakpoint matching {spec} or all with '*'
'e' | 'exit'                 - Stop flow execution and exit debugger
'h' | 'help'                 - Display this help message
'i' | 'inspect'              - Run a series of defined 'inspections' to check status of flow
'l' | 'list'                 - List all breakpoints
'p' | 'print' [n]            - Print the overall state, or state of process number 'n'
'r' | 'run' or 'reset'       - run the flow or if running already then reset the state to initial state
's' | 'step' [n]             - Step over the next 'n' jobs (default = 1) then break
'q' | 'quit'                 - Stop flow execution and exit debugger
";

/*
    A simple CLI (i.e. stdin and stdout) debug client that implements the DebugClient trait
    defined in the flowrlib library.
*/
pub struct CLIDebugClient {}

impl CLIDebugClient {
    pub fn start(mut connection: DebugClientConnection) {
        let _ = connection.start();

        std::thread::spawn(move || {
            // let _ = connection.client_send(Ack); // client must start the protocol
            loop {
                match connection.client_recv() {
                    Ok(event) => {
                        if let Ok(response) = Self::process_event(event) {
                            let _ = connection.client_send(response);
                        }
                    }
                    Err(err) => {
                        error!("Error receiving event from debugger: {}", err);
                        // break;
                    }
                }
            }
        });
    }

    fn help() {
        println!("{}", HELP_STRING);
    }

    fn parse_command(input: &str) -> (&str, Option<Param>) {
        let parts: Vec<&str> = input.trim().split(' ').collect();
        let command = parts[0];

        if parts.len() > 1 {
            if parts[1] == "*" {
                return (command, Some(Param::Wildcard));
            }

            if let Ok(integer) = parts[1].parse::<usize>() {
                return (command, Some(Param::Numeric(integer)));
            }

            if parts[1].contains('/') { // is an output specified
                let sub_parts: Vec<&str> = parts[1].split('/').collect();
                if let Ok(source_process_id) = sub_parts[0].parse::<usize>() {
                    return (command, Some(Param::Output((source_process_id, sub_parts[1].to_string()))));
                }
            } else if parts[1].contains(':') { // is an input specifier
                let sub_parts: Vec<&str> = parts[1].split(':').collect();
                match (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
                    (Ok(dest_process_id), Ok(dest_input_number)) =>
                        return (command, Some(Param::Input((dest_process_id, dest_input_number)))),
                    (_, _) => { /* couldn't parse the process and input numbers */ }
                }
            } else if parts[1].contains("->") { // is a block specifier
                let sub_parts: Vec<&str> = parts[1].split("->").collect();
                match (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
                    (Ok(blocked_process_id), Ok(blocking_process_id)) =>
                        return (command, Some(Param::Block((blocked_process_id, blocking_process_id)))),
                    (_, _) => { /* couldn't parse the process ids */ }
                }
            }
        }

        (command, None)
    }

    /*
        Wait for the user to input a valid debugger command then return it
     */
    fn get_user_command(job_number: usize) -> Result<Response> {
        loop {
            print!("Debug #{}> ", job_number);
            io::stdout().flush().chain_err(|| "Could not flush stdout")?;

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => return Ok(ExitDebugger),
                Ok(_n) => {
                    let (command, param) = Self::parse_command(&input);
                    match command {
                        "b" | "breakpoint" => return Ok(Breakpoint(param)),
                        "" | "c" | "continue" => return Ok(Continue),
                        "d" | "delete" => return Ok(Delete(param)),
                        "e" | "exit" => return Ok(ExitDebugger),
                        "h" | "help" => Self::help(),
                        "i" | "inspect" => return Ok(Inspect),
                        "l" | "list" => return Ok(List),
                        "p" | "print" => return Ok(Print(param)),
                        "r" | "run" | "reset" => return Ok(RunReset),
                        "s" | "step" => return Ok(Step(param)),
                        "q" | "quit" => return Ok(ExitDebugger),
                        _ => println!("Unknown debugger command '{}'\n", command)
                    }
                }
                Err(_) => bail!("Error reading debugger command")
            }
        }
    }

    /*
        This processes an event received from the server debugger in the local client that has stdio

        These are events generated by the remote debugger without any previous request from the debug client
    */
    pub fn process_event(event: Event) -> Result<Response> {
        match event {
            JobCompleted(job_id, function_id, opt_output) => {
                println!("Job #{} completed by Function #{}", job_id, function_id);
                if let Some(output) = opt_output {
                    println!("\tOutput value: '{}'", &output);
                }
            }
            PriorToSendingJob(job_id, function_id) =>
                println!("About to send Job #{} to Function #{}", job_id, function_id),
            BlockBreakpoint(block) =>
                println!("Block breakpoint: {:?}", block),
            DataBreakpoint(source_process_id, output_route, value,
                           destination_id, input_number) =>
                println!("Data breakpoint: Function #{}{}    ----- {} ----> Function #{}:{}",
                         source_process_id, output_route, value,
                         destination_id, input_number),
            Panic(message, jobs_created) => {
                println!("Function panicked after {} jobs created: {}", jobs_created, message);
                return Self::get_user_command(jobs_created);
            }
            JobError(job) => {
                println!("Error occurred executing a Job: \n'{}'", job);
                return Self::get_user_command(job.job_id);
            }
            EnteringDebugger =>
                println!("Entering Debugger. Use 'h' or 'help' for help on commands"),
            ExitingDebugger =>
                println!("Debugger is exiting"),
            ExecutionStarted =>
                println!("Running flow"),
            ExecutionEnded =>
                println!("Flow has completed"),
            Deadlock(message) =>
                println!("Deadlock detected{}", message),
            SendingValue(source_process_id, value, destination_id, input_number) =>
                println!("Function #{} sending '{}' to {}:{}",
                         source_process_id, value, destination_id, input_number),
            Event::Error(error_message) =>
                println!("{}", error_message),
            Message(message) =>
                println!("{}", message),
            Resetting =>
                println!("Resetting state"),
            WaitingForCommand(job_id) => return Self::get_user_command(job_id),
            Event::Invalid => {}
        }

        Ok(Ack)
    }
}