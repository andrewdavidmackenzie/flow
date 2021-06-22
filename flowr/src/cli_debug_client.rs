use std::io;
use std::io::Write;

use log::error;

use flowrlib::client_server::DebugClientConnection;
use flowrlib::debug::{Event, Event::*, Param, Response, Response::*};

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
'i' | 'inspect' [n]          - Inspect the overall state, or the state of function number 'n'
'l' | 'list'                 - List all breakpoints
'q' | 'quit'                 - Stop flow execution and exit debugger
'r' | 'run' or 'reset'       - run the flow or if running already then reset the state to initial state
's' | 'step' [n]             - Step over the next 'n' jobs (default = 1) then break
'v' | 'validate'             - Validate the state of the flow by running a series of checks
";

/*
    A simple CLI (i.e. stdin and stdout) debug client that implements the DebugClient trait
    defined in the flowrlib library.
*/
pub struct CliDebugClient {}

impl CliDebugClient {
    pub fn start(mut connection: DebugClientConnection) {
        let _ = connection.start();

        std::thread::spawn(move || loop {
            match connection.client_recv() {
                Ok(event) => {
                    if let Ok(response) = Self::process_event(event) {
                        let _ = connection.client_send(response);
                    }
                }
                Err(err) => {
                    error!("Error receiving event from debugger: {}", err);
                    break;
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

            if parts[1].contains('/') {
                // is an output specified
                let sub_parts: Vec<&str> = parts[1].split('/').collect();
                if let Ok(source_process_id) = sub_parts[0].parse::<usize>() {
                    return (
                        command,
                        Some(Param::Output((source_process_id, sub_parts[1].to_string()))),
                    );
                }
            } else if parts[1].contains(':') {
                // is an input specifier
                let sub_parts: Vec<&str> = parts[1].split(':').collect();
                match (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
                    (Ok(destination_process_id), Ok(destination_input_number)) => {
                        return (
                            command,
                            Some(Param::Input((
                                destination_process_id,
                                destination_input_number,
                            ))),
                        )
                    }
                    (_, _) => { /* couldn't parse the process and input numbers */ }
                }
            } else if parts[1].contains("->") {
                // is a block specifier
                let sub_parts: Vec<&str> = parts[1].split("->").collect();
                match (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
                    (Ok(blocked_process_id), Ok(blocking_process_id)) => {
                        return (
                            command,
                            Some(Param::Block((blocked_process_id, blocking_process_id))),
                        )
                    }
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
            io::stdout()
                .flush()
                .chain_err(|| "Could not flush stdout")?;

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
                        "i" | "inspect" => {
                            match param {
                                None => return Ok(InspectFunction(None)),
                                Some(Param::Numeric(_)) => return Ok(InspectFunction(param)),
                                _ => println!("Unsupported format for inspect command. Use 'h' or 'help' command for help")
                            }
                        },
                        "l" | "list" => return Ok(List),
                        "q" | "quit" => return Ok(ExitDebugger),
                        "r" | "run" | "reset" => return Ok(RunReset),
                        "s" | "step" => return Ok(Step(param)),
                        "v" | "validate" => return Ok(Validate),
                        _ => println!("Unknown debugger command '{}'\n", command),
                    }
                }
                Err(_) => bail!("Error reading debugger command"),
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
            PriorToSendingJob(job_id, function_id) => {
                println!("About to send Job #{} to Function #{}", job_id, function_id)
            }
            BlockBreakpoint(block) => println!("Block breakpoint: {:?}", block),
            DataBreakpoint(
                source_process_id,
                output_route,
                value,
                destination_id,
                input_number,
            ) => println!(
                "Data breakpoint: Function #{}{}    ----- {} ----> Function #{}:{}",
                source_process_id, output_route, value, destination_id, input_number
            ),
            Panic(message, jobs_created) => {
                println!(
                    "Function panicked after {} jobs created: {}",
                    jobs_created, message
                );
                return Self::get_user_command(jobs_created);
            }
            JobError(job) => {
                println!("Error occurred executing a Job: \n'{}'", job);
                return Self::get_user_command(job.job_id);
            }
            EnteringDebugger => {
                println!("Entering Debugger. Use 'h' or 'help' for help on commands")
            }
            ExitingDebugger => println!("Debugger is exiting"),
            ExecutionStarted => println!("Running flow"),
            ExecutionEnded => println!("Flow has completed"),
            Deadlock(message) => println!("Deadlock detected{}", message),
            SendingValue(source_process_id, value, destination_id, input_number) => println!(
                "Function #{} sending '{}' to {}:{}",
                source_process_id, value, destination_id, input_number
            ),
            Event::Error(error_message) => println!("{}", error_message),
            Message(message) => println!("{}", message),
            Resetting => println!("Resetting state"),
            WaitingForCommand(job_id) => return Self::get_user_command(job_id),
            Event::Invalid => {}
            FunctionState((function, state)) => {
                print!("{}", function);
                println!("\tState: {:?}", state);
            }
            FunctionStates(function_state_vec) => {
                for (function, state) in function_state_vec {
                    print!("{}", function);
                    println!("\tState: {:?}\n", state);
                }
            }
        }

        Ok(Ack)
    }

    // #[cfg(feature = "debugger")]
    // pub fn display_state(&self, function_id: usize) -> String {
    //     let function_state = self.get_state(function_id);
    //     let mut output = format!("\tState: {:?}\n", function_state);
    //
    //     if function_state == State::Running {
    //         output.push_str(&format!(
    //             "\t\tJob Numbers Running: {:?}\n",
    //             self.running.get_vec(&function_id)
    //         ));
    //     }
    //
    //     for block in &self.blocks {
    //         if block.blocked_flow_id == function_id {
    //             output.push_str(&format!("\t{:?}\n", block));
    //         } else if block.blocking_id == function_id {
    //             output.push_str(&format!(
    //                 "\tBlocking #{}:{} <-- Blocked #{}\n",
    //                 block.blocking_id, block.blocking_io_number, block.blocked_id
    //             ));
    //         }
    //     }
    //
    //     output
    // }
}

// #[cfg(any(feature = "debugger"))]
// #[test]
// fn debugger_can_display_run_state() {
//     let f_a = super::test_function_a_to_b();
//     let f_b = super::test_function_b_init();
//     let functions = vec![f_b, f_a];
//     let submission = Submission::new(
//         &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
//         1,
//         true,
//     );
//     let mut state = RunState::new(&functions, submission);
//
//     // Event
//     state.init();
//
//     // Test
//     assert_eq!(2, state.num_functions(), "There should be 2 functions");
//     assert_eq!(
//         State::Blocked,
//         state.get_state(0),
//         "f_a should be in Blocked state"
//     );
//     assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
//     assert_eq!(
//         1,
//         state.number_jobs_ready(),
//         "There should be 1 job running"
//     );
//     let mut blocked = HashSet::new();
//     blocked.insert(0);
//
//     // Test
//     assert_eq!(
//         &blocked,
//         state.get_blocked(),
//         "Function with ID = 1 should be in 'blocked' list"
//     );
//     state.display_state(0);
//     state.display_state(1);
//
//     // Event
//     let job = state.next_job().expect("Couldn't get next job");
//     state.start(&job);
//
//     // Test
//     assert_eq!(State::Running, state.get_state(1), "f_b should be Running");
//     assert_eq!(
//         1,
//         state.number_jobs_running(),
//         "There should be 1 job running"
//     );
//     state.display_state(1);
// }
