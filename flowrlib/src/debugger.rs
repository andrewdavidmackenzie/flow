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
        let mut input = String::new();
        loop {
            self.client.display("Debug> ");
            match self.client.read_input(&mut input) {
                Ok(_n) => {
                    // parse command
                    // if continue, then return
                    return;
                }
                Err(_) => {}
            };
        }
    }
}
