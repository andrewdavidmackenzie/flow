use debug_client::DebugClient;
use std::process::exit;
use run_state::RunState;
use std::collections::HashSet;
use serde_json::Value as JsonValue;

pub struct Debugger {
    pub client: &'static DebugClient,
    input_breakpoints: HashSet<(usize, usize)>,
    output_breakpoints: HashSet<(usize, String)>,
    break_at_invocation: usize,
    process_breakpoints: HashSet<usize>,
}

const HELP_STRING: &str = "Debugger commands:
'b' | 'breakpoint' {spec}    - Set a breakpoint on a process, an output or an input using spec:
                                - process_id
                                - source_id/output_route ('source_id/' for default output route)
                                - destination_id:input_number
ENTER | 'c' | 'continue'     - Continue execution until next breakpoint
'd' | 'delete' {spec} or '*' - Delete the breakpoint matching {spec} or all with '*'
'e' | 'exit'                 - Stop flow execution and exit
'h' | 'help'                 - Display this help message
'lb'| 'breakpoints'          - List all the currently set breakpoints
'p' | 'print' [n]            - Print the overall state, or state of process number 'n'
'r' | 'reset'                - reset the state back to initial state after loading
's' | 'step' [n]             - Step over the next 'n' process executions (default = 1) then break
";

enum Param {
    Wildcard,
    Numeric(usize),
    Output((usize, String)),
    Input((usize, usize)),
}

impl Debugger {
    pub fn new(client: &'static DebugClient) -> Self {
        Debugger {
            client,
            input_breakpoints: HashSet::<(usize, usize)>::new(),
            output_breakpoints: HashSet::<(usize, String)>::new(),
            break_at_invocation: 0,
            process_breakpoints: HashSet::<usize>::new(),
        }
    }

    /*
        return true if the debugger requests that we display the output of the next dispatch
    */
    pub fn check(&mut self, state: &mut RunState, next_process_id: usize) -> (bool, bool) {
        if self.break_at_invocation == state.dispatches() {
            return self.command_loop(state);
        }

        if self.process_breakpoints.contains(&next_process_id) {
            self.print(state, Some(Param::Numeric(next_process_id)));
            return self.command_loop(state);
        }

        (false, false)
    }

    pub fn watch_data(&mut self, state: &mut RunState, source_process_id: usize, output_route: &String,
                      value: &JsonValue, destination_id: usize, input_number: usize) {
        if self.output_breakpoints.contains(&(source_process_id, output_route.to_string())) ||
            self.input_breakpoints.contains(&(destination_id, input_number)) {
            self.client.display(&format!("Data breakpoint: Process #{}/{}    ----- {} ----> Process #{}:{}\n",
                                         source_process_id, output_route, value,
                                         destination_id, input_number));
            self.command_loop(state);
        }
    }

    pub fn panic(&mut self, state: &mut RunState, _cause: Box<std::any::Any + std::marker::Send>,
                 id: usize, name: &str, inputs: Vec<Vec<JsonValue>>) {
        self.client.display(
            &format!("Panic occurred in implementation. Entering debugger\nProcess #{} '{}' with inputs: {:?}\n",
        id, name, inputs));
        self.command_loop(state);
    }

    pub fn end(&mut self, state: &mut RunState,) -> (bool, bool) {
        self.client.display("Execution has ended\n");
        self.command_loop(state)
    }

    /*
        Return values are (display_output of next invocation, reset execution)
    */
    pub fn command_loop(&mut self, state: &mut RunState) -> (bool, bool) {
        loop {
            self.client.display(&format!("Debug #{}> ", state.dispatches()));
            let mut input = String::new();
            match self.client.read_input(&mut input) {
                Ok(_n) => {
                    let (command, param) = Self::parse_command(&input);
                    match command {
                        "b" | "breakpoint" => self.add_breakpoint(state, param),
                        "" | "c" | "continue" => return (false, false),
                        "d" | "delete" => self.delete_breakpoint(param),
                        "e" | "exit" => exit(1),
                        "h" | "help" => self.help(),
                        "lb" | "breakpoints" => self.list_breakpoints(),
                        "p" | "print" => self.print(state, param),
                        "r" | "reset" => {
                            self.client.display("Resetting state\n");
                            state.reset();
                            return (false, true);
                        },
                        "s" | "step" => {
                            self.step(state, param);
                            return (true, false);
                        }
                        _ => self.client.display(&format!("Unknown debugger command '{}'\n", command))
                    }
                }
                Err(_) => self.client.display(&format!("Error reading debugger command\n"))
            };
        }
    }

    fn delete_breakpoint(&mut self, param: Option<Param>) {
        match param {
            None => self.client.display("No process id specified\n"),
            Some(Param::Numeric(process_number)) => {
                if self.process_breakpoints.remove(&process_number) {
                    self.client.display(
                        &format!("Breakpoint on process #{} was deleted\n", process_number));
                } else {
                    self.client.display("No breakpoint number '{}' exists\n");
                }
            }
            Some(Param::Input((dest_id, input_number))) => {
                self.input_breakpoints.remove(&(dest_id, input_number));
            }
            Some(Param::Output((source_id, source_output_route))) => {
                self.output_breakpoints.remove(&(source_id, source_output_route));
            }
            Some(Param::Wildcard) => {
                self.output_breakpoints.clear();
                self.input_breakpoints.clear();
                self.process_breakpoints.clear();
                self.client.display("Deleted all breakpoints\n");
            }
        }
    }

    fn list_breakpoints(&self) {
        if !self.process_breakpoints.is_empty() {
            self.client.display("Process Breakpoints: \n");
            for process_id in &self.process_breakpoints {
                self.client.display(&format!("\tProcess #{}\n", process_id));
            }
        }

        if !self.output_breakpoints.is_empty() {
            self.client.display("Output Breakpoints: \n");
            for (process_id, route) in &self.output_breakpoints {
                self.client.display(&format!("\tOutput #{}/{}\n", process_id, route));
            }
        }

        if !self.input_breakpoints.is_empty() {
            self.client.display("Input Breakpoints: \n");
            for (process_id, input_number) in &self.input_breakpoints {
                self.client.display(&format!("\tInput #{}:{}\n", process_id, input_number));
            }
        }
    }

    fn print_process(&self, state: &RunState, process_id: usize) {
        let process_arc = state.get(process_id);
        let mut process_lock = process_arc.try_lock();

        if let Ok(ref mut process) = process_lock {
            self.client.display(&format!("{}", process))
            // TODO print out information about what state it is in etc
        } else {
            self.client.display(&format!("Process #{} locked, skipping\n", process_id))
        }
    }

    fn print_all_processes(&self, state: &RunState) {
        for id in 0..state.num_processes() {
            self.print_process(state, id);
        }
    }

    fn print(&self, state: &RunState, param: Option<Param>) {
        match param {
            None => state.print(),
            Some(Param::Numeric(process_id)) |
            Some(Param::Input((process_id, _))) => self.print_process(state, process_id),
            Some(Param::Wildcard) => self.print_all_processes(state),
            Some(Param::Output(_)) => self.client.display(
                "Cannot display the output of a process until it is executed. \
                Set a breakpoint on the process by id and then step over it")
        }
    }

    fn step(&mut self, state: &RunState, steps: Option<Param>) {
        match steps {
            None => self.break_at_invocation = state.dispatches() + 1,
            Some(Param::Numeric(steps)) => self.break_at_invocation = state.dispatches() + steps,
            _ => self.client.display("Did not understand step command parameter\n")
        }
    }

    fn add_breakpoint(&mut self, state: &RunState, param: Option<Param>) {
        match param {
            None => self.client.display("'break' command must specify a process id to break on"),
            Some(Param::Numeric(process_id)) => {
                if process_id > state.num_processes() {
                    self.client.display(
                        &format!("There is no process with id '{}' to set a breakpoint on\n", process_id));
                } else {
                    self.process_breakpoints.insert(process_id);
                    self.client.display(
                        &format!("Set process breakpoint on Process #{}\n", process_id));
                }
            }
            Some(Param::Input((dest_id, input_number))) => {
                self.client.display(
                    &format!("Set data breakpoint on process #{} receiving data on input: {}\n", dest_id, input_number));
                self.input_breakpoints.insert((dest_id, input_number));
            }
            Some(Param::Output((source_id, source_output_route))) => {
                self.client.display(
                    &format!("Set data breakpoint on process #{} sending data via output/{}\n", source_id, source_output_route));
                self.output_breakpoints.insert((source_id, source_output_route));
            }
            Some(Param::Wildcard) => self.client.display("To break on every process, you can just single step using 's' command\n")
        }
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
                    (_, _) => { /* couldn't parse the process and input n umbers*/ }
                }
            }
        }

        (command, None)
    }

    fn help(&self) {
        self.client.display(HELP_STRING);
    }
}
