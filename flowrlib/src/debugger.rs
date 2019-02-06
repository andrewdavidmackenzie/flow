use debug_client::DebugClient;
use std::process::exit;
use run_state::RunState;
use process::Process;
use std::collections::HashSet;

pub struct Debugger {
    pub client: &'static DebugClient,
    stop_at: usize,
    breakpoints: HashSet<usize>
}

const HELP_STRING: &str = "Debugger commands:
'b' | 'breakpoint' n     - Set a breakpoint on process with id 'n'
ENTER | 'c' | 'continue' - Continue execution until next breakpoint
'd' | 'delete' n         - Delete the breakpoint on process number 'n'
'e' | 'exit'             - Stop flow execution and exit
'h' | 'help'             - Display this help message
'lb'| 'breakpoints'      - List all the currently set breakpoints
'p' | 'print' [n]        - Print the overall state, or state of process number 'n'
's' | 'step' [n]         - Step over the next 'n' process executions (default = 1) then break
";

impl Debugger {
    pub fn new(client: &'static DebugClient) -> Self {
        Debugger {
            client,
            stop_at: 0,
            breakpoints: HashSet::<usize>::new()
        }
    }

    /*
        return true if the debugger requests that we display the output of the next dispatch
    */
    pub fn check(&mut self, state: &RunState, next_process_id: usize) -> bool {
        if self.stop_at == state.dispatches()  {
            return self.enter(state, next_process_id, false);
        }

        if self.breakpoints.contains(&next_process_id) {
            return self.enter(state, next_process_id, true);
        }

        false
    }

    fn enter(&mut self, state: &RunState, next_process_id: usize, print_next: bool) -> bool {
        if print_next {
            self.print(state, Some(next_process_id));
        }

        loop {
            self.client.display(&format!("Debug #{}> ", self.stop_at));
            let mut input = String::new();
            match self.client.read_input(&mut input) {
                Ok(_n) => {
                    let (command, param) = Self::parse_command(&input);
                    match command {
                        "b" | "breakpoint" => self.breakpoint(state, param),
                        "" | "c" | "continue" => return false,
                        "d" | "delete" => self.delete(param),
                        "e" | "exit" => exit(1),
                        "h" | "help" => self.help(),
                        "lb" | "breakpoints" => self.list_breakpoints(),
                        "p" | "print" => self.print(state, param),
                        "s" | "step" => {
                            self.step(state, param);
                            return true;
                        }
                        _ => self.client.display(&format!("Unknown debugger command '{}'\n", command))
                    }
                }
                Err(_) => self.client.display(&format!("Error reading debugger command\n"))
            };
        }
    }

    fn delete(&mut self, param: Option<usize>) {
        match param {
            None =>  self.client.display("No process id specified\n"),
            Some(process_number) => {
                if self.breakpoints.remove(&process_number) {
                    self.client.display(
                        &format!("Breakpoint on process #{} was deleted\n", process_number));
                } else {
                    self.client.display("No breakpoint number '{}' exists\n");
                }
            }
        }
    }

    fn list_breakpoints(&self) {
        if self.breakpoints.is_empty() {
            self.client.display("No breakpoints set\n");
            return;
        }

        self.client.display("Breakpoints: \n");
        for process_id in &self.breakpoints {
            self.client.display(&format!("\tProcess #{}\n", process_id));
        }
    }

    fn print(&self, state: &RunState, param: Option<usize>) {
        match param {
            None => state.print(),
            Some(process_number) => {
                let process_arc = state.get(process_number);
                let process: &mut Process = &mut *process_arc.lock().unwrap();
                self.client.display(&format!("{}", process));
                // TODO print out information about what state it is in etc
            }
        }
    }

    fn step(&mut self, state: &RunState, steps: Option<usize>) {
        match steps {
            None => {
                self.stop_at = state.dispatches() + 1;
            },
            Some(steps) => {
                self.stop_at = state.dispatches() + steps;
            }
        }
    }

    fn breakpoint(&mut self, state: &RunState, param: Option<usize>) {
        match param {
            None => self.client.display("'break' command must specify a process id to break on"),
            Some(process_id) => {
                if process_id > state.num_processes() {
                    self.client.display(
                        &format!("There is no process with id '{}' to set a breakpoint on\n", process_id));
                } else {
                    self.breakpoints.insert(process_id);
                    self.client.display(
                        &format!("Breakpoint set on process with id '{}'\n", process_id));
                }
            }
        }
    }

    fn parse_command(input: &String) -> (&str, Option<usize>) {
        let parts: Vec<&str> = input.trim().split(' ').collect();
        let command = parts[0];
        let mut parameter = None;

        if parts.len() > 1 {
            match parts[1].parse::<usize>() {
                Ok(integer) => parameter = Some(integer),
                Err(_) => {}
            }
        }

        (command, parameter)
    }

    fn help(&self) {
        self.client.display(HELP_STRING);
    }
}
