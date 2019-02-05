use runlist::RunList;
use debug_client::DebugClient;
use std::process::exit;

pub struct Debugger {
    client: &'static DebugClient,
    pub stop_at: u32
}

impl Debugger {
    pub fn new(client: &'static DebugClient) -> Self {
        Debugger {
            client, stop_at:0
        }
    }

    pub fn enter(&self, run_list: &RunList) {
        loop {
            self.client.display("Debug> ");
            let mut input = String::new();
            match self.client.read_input(&mut input) {
                Ok(_n) => {
                    let parts : Vec<&str>= input.trim().split(' ').collect();
                    match parts[0] {
                        "e" | "exit" => exit(1),
                        "d" | "display" => run_list.print_state(),
                        "" | "c" | "continue" => {
                            return;
                        },
                        _ => {self.client.display(&format!("Unknown debugger command '{}'\n", parts[0]))}
                    }
                }
                Err(_) => {}
            };
        }
    }
}
