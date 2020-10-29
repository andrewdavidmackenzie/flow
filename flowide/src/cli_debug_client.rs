// use std::io;
// use std::io::Write;
//
// use flowrlib::debug::{Event, Event::{*}, Param, Response, Response::{*}};

// const HELP_STRING: &str = "Debugger commands:
// 'b' | 'breakpoint' {spec}    - Set a breakpoint on a function (by id), an output or an input using spec:
//                                 - function_id
//                                 - source_id/output_route ('source_id/' for default output route)
//                                 - destination_id:input_number
//                                 - blocked_process_id->blocking_process_id
// ENTER | 'c' | 'continue'     - Continue execution until next breakpoint
// 'd' | 'delete' {spec} or '*' - Delete the breakpoint matching {spec} or all with '*'
// 'e' | 'exit'                 - Stop flow execution and exit debugger
// 'h' | 'help'                 - Display this help message
// 'i' | 'inspect'              - Run a series of defined 'inspections' to check status of flow
// 'l' | 'list'                 - List all breakpoints
// 'p' | 'print' [n]            - Print the overall state, or state of process number 'n'
// 'r' | 'run' or 'reset'       - run the flow or if running already then reset the state to initial state
// 's' | 'step' [n]             - Step over the next 'n' jobs (default = 1) then break
// 'q' | 'quit'                 - Stop flow execution and exit debugger
// ";
//
// /*
//     A simple CLI (i.e. stdin and stdout) debug client that implements the DebugClient trait
//     defined in the flowrlib library.
// */
// pub struct CLIDebugClient {
//
// }
//
// fn help() {
//     println!("{}", HELP_STRING);
// }
//
// fn parse_command(input: &str) -> (&str, Option<Param>) {
//     let parts: Vec<&str> = input.trim().split(' ').collect();
//     let command = parts[0];
//
//     if parts.len() > 1 {
//         if parts[1] == "*" {
//             return (command, Some(Param::Wildcard));
//         }
//
//         if let Ok(integer) = parts[1].parse::<usize>() {
//             return (command, Some(Param::Numeric(integer)));
//         }
//
//         if parts[1].contains('/') { // is an output specified
//             let sub_parts: Vec<&str> = parts[1].split('/').collect();
//             if let Ok(source_process_id) = sub_parts[0].parse::<usize>() {
//                 return (command, Some(Param::Output((source_process_id, sub_parts[1].to_string()))));
//             }
//         } else if parts[1].contains(':') { // is an input specifier
//             let sub_parts: Vec<&str> = parts[1].split(':').collect();
//             match (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
//                 (Ok(dest_process_id), Ok(dest_input_number)) =>
//                     return (command, Some(Param::Input((dest_process_id, dest_input_number)))),
//                 (_, _) => { /* couldn't parse the process and input numbers */ }
//             }
//         } else if parts[1].contains("->") { // is a block specifier
//             let sub_parts: Vec<&str> = parts[1].split("->").collect();
//             match (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
//                 (Ok(blocked_process_id), Ok(blocking_process_id)) =>
//                     return (command, Some(Param::Block((blocked_process_id, blocking_process_id)))),
//                 (_, _) => { /* couldn't parse the process ids */ }
//             }
//         }
//     }
//
//     (command, None)
// }
//
// fn get_user_command(job_number: usize) -> Response {
//     loop {
//         print!("Debug #{}> ", job_number);
//         io::stdout().flush().unwrap();
//
//         let mut input = String::new();
//         match io::stdin().read_line(&mut input) {
//             Ok(0) => return ExitDebugger,
//             Ok(_n) => {
//                 let (command, param) = parse_command(&input);
//                 match command {
//                     "b" | "breakpoint" => return Breakpoint(param),
//                     "" | "c" | "continue" => return Continue,
//                     "d" | "delete" => return Delete(param),
//                     "e" | "exit" => return ExitDebugger,
//                     "h" | "help" => help(),
//                     "i" | "inspect" => return Inspect,
//                     "l" | "list" => return List,
//                     "p" | "print" => return Print(param),
//                     "r" | "run" | "reset" => return RunReset,
//                     "s" | "step" => return Step(param),
//                     "q" | "quit" => return ExitDebugger,
//                     _ => println!("Unknown debugger command '{}'\n", command)
//                 }
//             }
//             Err(_) => println!("Error reading debugger command\n")
//         }
//     }
// }
//
// impl CLIDebugClient {
//     fn process_event(&self, event: Event) {
//         match event {
//             JobCompleted(job_id, function_id, opt_output) => {
//                 println!("Job #{} completed by Function #{}", job_id, function_id);
//                 if let Some(output) = opt_output {
//                     println!("\tOutput value: '{}'", &output);
//                 }
//             }
//             PriorToSendingJob(job_id, function_id) =>
//                 println!("About to send Job #{} to Function #{}", job_id, function_id),
//             BlockBreakpoint(block) =>
//                 println!("Block breakpoint: {:?}", block),
//             DataBreakpoint(source_process_id, output_route, value,
//                            destination_id, input_number) =>
//                 println!("Data breakpoint: Function #{}{}    ----- {} ----> Function #{}:{}",
//                          source_process_id, output_route, value,
//                          destination_id, input_number),
//             Panic(message, jobs_created) =>
//                 println!("Function panicked after {} jobs created: {}", jobs_created, message),
//             JobError(job) =>
//                 println!("Error occurred executing a Job: \n'{:?}'", job),
//             ExecutionStarted =>
//                 println!("Running flow"),
//             ExecutionEnded =>
//                 println!("Flow has completed"),
//             Deadlock(message) =>
//                 println!("Deadlock detected{}", message),
//             SendingValue(source_process_id, value, destination_id, input_number) =>
//                 println!("Function #{} sending '{}' to {}:{}",
//                          source_process_id, value, destination_id, input_number),
//             Event::Error(error_message) =>
//                 println!("{}", error_message),
//             Message(message) =>
//                 println!("{}", message),
//             Resetting =>
//                 println!("Resetting state"),
//             EnteringDebugger =>
//                 println!("Entering Debugger. Use 'h' or 'help' for help on commands"),
//             ExitingDebugger =>
//                 println!("Debugger is exiting"),
//             WaitingForCommand(job_id) => {
//                 let _response = get_user_command(job_id);
//                 // self.client.send(response);
//             }
//         }
//     }
//
//     pub fn send_event(&self, event: Event) {
//         self.process_event(event)
//     }
// }