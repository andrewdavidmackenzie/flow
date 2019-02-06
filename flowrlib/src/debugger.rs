use debug_client::DebugClient;
use std::process::exit;
use runlist::State;

pub struct Debugger {
    client: &'static DebugClient,
    pub stop_at: u32
}

const HELP_STRING: &str = "Debugger commands:
'b' | 'break' n          - Set a breakpoint before dispatch 'n'
ENTER | 'c' | 'continue' - Continue execution until next breakpoint
'e' | 'exit'             - Stop flow execution and exit
'd' | 'display' [n]      - Display the overall state, or that or process number 'n'
'h' | 'help'             - Display this help message
's' | 'step' [n]         - Step over the next 'n' process executions (default = 1) then break
";

impl Debugger {
    pub fn new(client: &'static DebugClient) -> Self {
        Debugger {
            client, stop_at:0
        }
    }

    pub fn check(&mut self, state: &State) {
        if self.stop_at == state.dispatches() {
            self.client.display(&format!("Break on dispatch '{}'\n", self.stop_at));
            self.enter(state);
        }
    }

    fn enter(&mut self, state: &State) {
        loop {
            self.client.display("Debug> ");
            let mut input = String::new();
            match self.client.read_input(&mut input) {
                Ok(_n) => {
                    let (command, param) = Self::parse_command(&input);
                    match command {
                        "b" | "break" => {
                            match param {
                                None => self.client.display("'break' command must specify a dispatch to break on"),
                                Some(dispatch) => {
                                    if state.dispatches() >= dispatch {
                                        self.client.display("Dispatch '{}' has already occurred, cannot set breakpoint there\n")
                                    } else {
                                        self.stop_at = dispatch;
                                        self.client.display(&format!("Breakpoint set on dispatch '{}'\n", dispatch));
                                    }
                                }
                            }
                        },
                        "e" | "exit" => exit(1),
                        "d" | "display" => state.print(),
                        "" | "c" | "continue" => {
                            return;
                        },
                        "s" | "step" => {
                            self.stop_at = state.dispatches() + 1;
                            return;
                        }
                        "h" | "help" => self.help(),
                        _ => self.client.display(&format!("Unknown debugger command '{}'\n", command))
                    }
                }
                Err(_) => self.client.display(&format!("Error reading debugger command\n"))
            };
        }
    }

    fn parse_command(input: &String) -> (&str, Option<u32>) {
        let parts : Vec<&str>= input.trim().split(' ').collect();
        let command = parts[0];
        let mut parameter = None;

        if parts.len() > 1 {
            match parts[1].parse::<u32>() {
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
