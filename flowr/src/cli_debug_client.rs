use log::error;
use rustyline::Editor;

use flowrlib::client_server::ClientConnection;
use flowrlib::debug_messages::{
    DebugClientMessage, DebugClientMessage::*, DebugServerMessage, DebugServerMessage::*, Param,
};
use flowrlib::run_state::{RunState, State};

use crate::errors::*;

const FLOWR_HISTORY_FILENAME: &str = ".flowr_history";

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
pub struct CliDebugClient {
    connection: ClientConnection<'static, DebugServerMessage, DebugClientMessage>,
    editor: Editor<()>,
}

impl CliDebugClient {
    /// Create a new debug client accepting the debug connection
    pub fn new(
        connection: ClientConnection<'static, DebugServerMessage, DebugClientMessage>,
    ) -> Self {
        CliDebugClient {
            connection,
            editor: Editor::<()>::new(), // `()` can be used when no completer is required
        }
    }

    /// Start running the debug client event loop in a new thread
    pub fn event_loop_thread(mut self) {
        // Ignore error on first start-up due to no previous command history existing
        let _ = self.editor.load_history(FLOWR_HISTORY_FILENAME);

        let _ = std::thread::spawn(move || {
            self.debug_client_loop();
        });
    }

    /*
       Main debug client loop where events are received, processed and responses sent
    */
    fn debug_client_loop(&mut self) {
        // Send an first message to initialize the connection
        let _ = self
            .connection
            .send(DebugClientMessage::DebugClientStarting);

        // loop while? and avoid break?
        loop {
            match self.connection.receive() {
                Ok(debug_server_message) => {
                    if let Ok(response) = self.process_event(debug_server_message) {
                        let _ = self.connection.send(response);
                    }
                }
                Err(err) => {
                    error!("Error receiving event from debugger: {}", err);
                    break;
                }
            }
        }

        let _ = self.editor.save_history(FLOWR_HISTORY_FILENAME);
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
                        Some(Param::Output((
                            source_process_id,
                            format!("/{}", sub_parts[1].to_string()),
                        ))),
                    );
                }
            } else if parts[1].contains(':') {
                // is an input specifier
                let sub_parts: Vec<&str> = parts[1].split(':').collect();
                match (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
                    (Ok(destination_function_id), Ok(destination_input_number)) => {
                        return (
                            command,
                            Some(Param::Input((
                                destination_function_id,
                                destination_input_number,
                            ))),
                        )
                    }
                    (_, _) => { /* couldn't parse the process and input numbers */ }
                }
            } else if parts[1].contains("->") {
                // is a block specifier
                let sub_parts: Vec<&str> = parts[1].split("->").collect();
                let source = sub_parts[0].parse::<usize>().ok();
                let destination = sub_parts[1].parse::<usize>().ok();
                return (command, Some(Param::Block((source, destination))));
            }
        }

        (command, None)
    }

    /*
       Wait for the user to input a valid debugger command then return the corresponding response
       that should be sent to the debug server
    */
    fn get_user_command(&mut self, job_number: usize) -> Result<DebugClientMessage> {
        loop {
            match self.editor.readline(&format!("Debug #{}> ", job_number)) {
                Ok(line) => {
                    let (command, param) = Self::parse_command(&line);
                    if let Some(response) = self.get_server_command(command, param) {
                        self.editor.add_history_entry(&line);
                        return Ok(response);
                    }
                }
                Err(_) => return Ok(ExitDebugger), // Includes CONTROL-C and CONTROL-D exits
            }
        }
    }

    /*
       Determine what response should be sent to the server for the input command, or none.
       Some commands act locally at the client (such as "help") and don't generate any response
       to send to the server.
       Likewise command parsing errors shouldn't generate a response to send to the server.
    */
    fn get_server_command(
        &mut self,
        command: &str,
        param: Option<Param>,
    ) -> Option<DebugClientMessage> {
        return match command {
            "b" | "breakpoint" => Some(Breakpoint(param)),
            "" | "c" | "continue" => Some(Continue),
            "d" | "delete" => Some(Delete(param)),
            "e" | "exit" => Some(ExitDebugger),
            "h" | "help" => {
                Self::help();
                self.editor.add_history_entry(command);
                None
            }
            "i" | "inspect" => match param {
                None => Some(Inspect),
                Some(Param::Numeric(function_id)) => Some(InspectFunction(function_id)),
                Some(Param::Input((function_id, input_number))) => {
                    Some(InspectInput(function_id, input_number))
                }
                Some(Param::Output((function_id, sub_route))) => {
                    Some(InspectOutput(function_id, sub_route))
                }
                Some(Param::Block((source_function_id, destination_function_id))) => {
                    Some(InspectBlock(source_function_id, destination_function_id))
                }
                _ => {
                    println!("Unsupported format for 'inspect' command. Use 'h' or 'help' command for help");
                    None
                }
            },
            "l" | "list" => Some(List),
            "q" | "quit" => Some(ExitDebugger),
            "r" | "run" | "reset" => Some(RunReset),
            "s" | "step" => Some(Step(param)),
            "v" | "validate" => Some(Validate),
            _ => {
                println!("Unknown debugger command '{}'\n", command);
                None
            }
        };
    }

    /*
        This processes an event received from the server debugger in the local client that has stdio

        These are events generated by the remote debugger without any previous request from the debug client
    */
    fn process_event(&mut self, event: DebugServerMessage) -> Result<DebugClientMessage> {
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
                return self.get_user_command(jobs_created);
            }
            JobError(job) => {
                println!("Error occurred executing a Job: \n'{}'", job);
                return self.get_user_command(job.job_id);
            }
            EnteringDebugger => println!(
                "Server is Entering Debugger. Use 'h' or 'help' for help on commands at the prompt"
            ),
            ExitingDebugger => println!("Debugger is exiting"),
            ExecutionStarted => println!("Running flow"),
            ExecutionEnded => println!("Flow has completed"),
            Deadlock(message) => println!("Deadlock detected{}", message),
            SendingValue(source_process_id, value, destination_id, input_number) => println!(
                "Function #{} sending '{}' to {}:{}",
                source_process_id, value, destination_id, input_number
            ),
            DebugServerMessage::Error(error_message) => println!("{}", error_message),
            Message(message) => println!("{}", message),
            Resetting => println!("Resetting state"),
            WaitingForCommand(job_id) => return self.get_user_command(job_id),
            DebugServerMessage::Invalid => {}
            FunctionState((function, state)) => {
                print!("{}", function);
                println!("\tState: {:?}", state);
            }
            OverallState(run_state) => Self::display_state(&run_state),
            InputState(input) => println!("{}", input),
            OutputState(output_connections) => {
                if output_connections.is_empty() {
                    println!("No output connections from that sub-route");
                } else {
                    for connection in output_connections {
                        println!("{}", connection)
                    }
                }
            }
            BlockState(blocks) => {
                if blocks.is_empty() {
                    println!("No blocks between functions matching the specification were found");
                }
                for block in blocks {
                    println!("{}", block);
                }
            }
        }

        Ok(Ack)
    }

    /*
       Display information to the user about the current RunState
    */
    fn display_state(run_state: &RunState) {
        println!("{}\n", run_state);

        println!("Functions:\n");

        for id in 0..run_state.num_functions() {
            print!("{}", run_state.get(id));
            let function_state = run_state.get_state(id);
            println!("\tState: {:?}\n", function_state);

            if function_state == State::Running {
                println!(
                    "\t\tJob Numbers Running: {:?}\n",
                    run_state.get_running().get_vec(&id)
                );
            }

            // print any blocked or blocking function information
            for block in run_state.get_blocks() {
                if block.blocked_flow_id == id {
                    println!("\t{:?}\n", block);
                } else if block.blocking_id == id {
                    println!(
                        "\tBlocking #{}:{} <- Blocked #{}\n",
                        block.blocking_id, block.blocking_io_number, block.blocked_id
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use serde_json::json;
    use url::Url;

    use flowcore::function::Function;
    use flowcore::input::Input;
    use flowcore::input::InputInitializer::Once;
    use flowcore::output_connection::{OutputConnection, Source};
    use flowrlib::coordinator::Submission;
    use flowrlib::run_state::{RunState, State};

    use super::*;

    fn test_function_b_init() -> Function {
        Function::new(
            #[cfg(feature = "debugger")]
            "fB",
            #[cfg(feature = "debugger")]
            "/fB",
            "file://fake/test",
            vec![Input::new(&Some(Once(json!(1))))],
            1,
            0,
            &[],
            false,
        )
    }

    fn test_function_a_to_b() -> Function {
        let connection_to_f1 = OutputConnection::new(
            Source::default(),
            1,
            0,
            0,
            0,
            false,
            "/fB".to_string(),
            #[cfg(feature = "debugger")]
            String::default(),
        );
        Function::new(
            #[cfg(feature = "debugger")]
            "fA",
            #[cfg(feature = "debugger")]
            "/fA",
            "file://fake/test",
            vec![Input::new(&Some(Once(json!(1))))],
            0,
            0,
            &[connection_to_f1],
            false,
        ) // outputs to fB:0
    }

    #[test]
    fn display_run_state() {
        let f_a = test_function_a_to_b();
        let f_b = test_function_b_init();
        let functions = vec![f_b, f_a];
        let submission = Submission::new(
            &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
            1,
            true,
        );
        let mut state = RunState::new(&functions, submission);

        // Event
        state.init();

        // Test
        assert_eq!(2, state.num_functions(), "There should be 2 functions");
        assert_eq!(
            State::Blocked,
            state.get_state(0),
            "f_a should be in Blocked state"
        );
        assert_eq!(State::Ready, state.get_state(1), "f_b should be Ready");
        assert_eq!(
            1,
            state.number_jobs_ready(),
            "There should be 1 job running"
        );
        let mut blocked = HashSet::new();
        blocked.insert(0);

        // Test
        assert_eq!(
            &blocked,
            state.get_blocked(),
            "Function with ID = 1 should be in 'blocked' list"
        );
        CliDebugClient::display_state(&state);

        // Event
        let job = state.next_job().expect("Couldn't get next job");
        state.start(&job);

        // Test
        assert_eq!(State::Running, state.get_state(1), "f_b should be Running");
        assert_eq!(
            1,
            state.number_jobs_running(),
            "There should be 1 job running"
        );

        CliDebugClient::display_state(&state);
    }
}
