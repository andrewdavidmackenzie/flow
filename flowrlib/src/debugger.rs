use runlist::RunList;
use debug_client::DebugClient;

pub struct Debugger {
    client: &'static DebugClient
}

impl Debugger {
    pub fn new(client: &'static DebugClient) -> Self {
        Debugger {
            client
        }
    }

    pub fn enter(&self, _run_list: &RunList) {
        loop {
            self.client.display("Debug> ");
            let mut input = String::new();
            match self.client.read_input(&mut input) {
                Ok(_n) => {
                    let parts : Vec<&str>= input.trim().split(' ').collect();
                    match parts[0] {
                        "c" | "continue" => {
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
