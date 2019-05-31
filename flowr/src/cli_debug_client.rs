use std::io;
use std::io::Write;

use flowrlib::debug_client::Command;
use flowrlib::debug_client::Command::{*};
use flowrlib::debug_client::DebugClient;
use flowrlib::debug_client::Param;

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
'r' | 'reset'                - reset the state back to initial state after loading
's' | 'step' [n]             - Step over the next 'n' jobs (default = 1) then break
'q' | 'quit'                 - Stop flow execution and exit debugger
";

/*
    A simple CLI (i.e. stdin and stdout) debug client that implements the DebugClient trait
    defined in the flowrlib library.
*/
pub struct CLIDebugClient {}


fn help() {
    println!("{}", HELP_STRING);
}

fn parse_command(input: &String) -> (&str, Option<Param>) {
    let parts: Vec<&str> = input.trim().split(' ').collect();
    let command = parts[0];

    if parts.len() > 1 {
        if parts[1] == "*" {
            return (command, Some(Param::Wildcard));
        }

        match parts[1].parse::<usize>() {
            Ok(integer) => return (command, Some(Param::Numeric(integer))),
            Err(_) => { /* not an integer - fall through */ }
        }

        if parts[1].contains("/") { // is an output specified
            let sub_parts: Vec<&str> = parts[1].split('/').collect();
            match sub_parts[0].parse::<usize>() {
                Ok(source_process_id) =>
                    return (command, Some(Param::Output((source_process_id, sub_parts[1].to_string())))),
                Err(_) => { /* couldn't parse source process id */ }
            }
        } else if parts[1].contains(":") { // is an input specifier
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
    Implement a client for the debugger that reads and writes to standard input and output
*/
impl DebugClient for CLIDebugClient {
    fn init(&self) {}

    fn display(&self, output: &str) {
        print!("{}", output);
        io::stdout().flush().unwrap();
    }

    fn read_input(&self, input: &mut String) -> io::Result<usize> {
        io::stdin().read_line(input)
    }

    fn get_command(&self, job_number: usize) -> Command {
        loop {
            self.display(&format!("Debug #{}> ", job_number));

            let mut input = String::new();
            match self.read_input(&mut input) {
                Ok(_n) => {
                    let (command, param) = parse_command(&input);
                    match command {
                        "b" | "breakpoint" => return Breakpoint(param),
                        "" | "c" | "continue" => return Continue,
                        "d" | "delete" => return Delete(param),
                        "e" | "exit" => return Exit,
                        "h" | "help" => help(),
                        "i" | "inspect" => return Inspect,
                        "l" | "list" => return List,
                        "p" | "print" => return Print(param),
                        "r" | "reset" => return Reset,
                        "s" | "step" => return Step(param),
                        "q" | "quit" => return Exit,
                        _ => println!("Unknown debugger command '{}'\n", command)
                    }
                }
                Err(_) => println!("Error reading debugger command\n")
            }
        }
    }
}