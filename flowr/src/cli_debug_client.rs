use log::error;
use rustyline::Editor;

use flowcore::errors::*;
use flowcore::model::runtime_function::RuntimeFunction;
use flowrlib::debug_command::DebugCommand;
use flowrlib::debug_command::DebugCommand::*;
use flowrlib::param::Param;
use flowrlib::run_state::{RunState, State};

use crate::client_server::ClientConnection;
use crate::debug_messages::{DebugServerMessage, DebugServerMessage::*};

const FLOWR_HISTORY_FILENAME: &str = ".flowr_history";

const HELP_STRING: &str = "Debugger commands:
'b' | 'breakpoint' {spec}    - Set a breakpoint on a function (by id), an output or an input using spec:
                                - function_id (integer)
                                - source_id/output_route ('source_id/' for default output route)
                                - destination_id:input_number
                                - blocked_process_id->blocking_process_id
'c' | 'continue'             - Continue execution until next breakpoint
'd' | 'delete' {spec} or '*' - Delete the breakpoint matching {spec} or all with '*'
'e' | 'exit'                 - Stop flow execution and exit debugger
'h' | 'help' | '?'           - Display this help message
'i' | 'inspect' [n]          - Inspect the overall state, or the function number 'n'
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
    connection: ClientConnection,
    editor: Editor<()>,
    last_command: String,
}

impl CliDebugClient {
    /// Create a new debug client accepting the debug connection
    pub fn new(connection: ClientConnection) -> Self {
        CliDebugClient {
            connection,
            editor: Editor::<()>::new(), // `()` can be used when no completer is required
            last_command: "".to_string(),
        }
    }

    /// Main debug client loop where events are received, processed and responses sent
    pub fn debug_client_loop(mut self) {
        // Ignore error on first start-up due to no previous command history existing
        let _ = self.editor.load_history(FLOWR_HISTORY_FILENAME);

        // Send an first message to initialize the connection
        let _ = self
            .connection
            .send(DebugCommand::DebugClientStarting);

        // loop while? and avoid break?
        loop {
            match self.connection.receive() {
                Ok(debug_server_message) => {
                    if let Ok(response) = self.process_message(debug_server_message) {
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

    fn parse_command(&self, mut input: String) -> Result<(String, String, Option<Param>)> {
        input = input.trim().to_string();
        if input.is_empty() && !self.last_command.is_empty() {
            input = self.last_command.clone();
            println!("Repeating last valid command: '{}'", input);
        }

        let parts: Vec<String> = input.split(' ').map(|s| s.to_string()).collect();
        let command = parts[0].to_string();

        if parts.len() > 1 {
            if parts[1] == "*" {
                return Ok((input, command, Some(Param::Wildcard)));
            }

            if let Ok(integer) = parts[1].parse::<usize>() {
                return Ok((input, command, Some(Param::Numeric(integer))));
            }

            if parts[1].contains('/') {
                // is an output specified
                let sub_parts: Vec<&str> = parts[1].split('/').collect();
                if let Ok(source_process_id) = sub_parts[0].parse::<usize>() {
                    return Ok((
                        input, command,
                        Some(Param::Output((
                            source_process_id,
                            format!("/{}", sub_parts[1]),
                        ))),
                    ));
                }
            } else if parts[1].contains(':') {
                // is an input specifier
                let sub_parts: Vec<&str> = parts[1].split(':').collect();
                if let (Ok(destination_function_id), Ok(destination_input_number)) = (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
                    return Ok((
                        input, command,
                        Some(Param::Input((
                            destination_function_id,
                            destination_input_number,
                        ))),
                    ));
                }
            } else if parts[1].contains("->") {
                // is a block specifier
                let sub_parts: Vec<&str> = parts[1].split("->").collect();
                let source = sub_parts[0].parse::<usize>().ok();
                let destination = sub_parts[1].parse::<usize>().ok();
                return Ok((input, command, Some(Param::Block((source, destination)))));
            }
        };

        Ok((input, command, None))
    }

    /*
       Wait for the user to input a valid debugger command then return the corresponding response
       that should be sent to the debug server
    */
    fn get_user_command(&mut self, job_number: usize) -> Result<DebugCommand> {
        loop {
            match self.editor.readline(&format!("Debug #{}> ", job_number)) {
                Ok(line) => {
                    match self.parse_command(line) {
                        Ok((line, command, param)) => {
                            if let Some(debugger_command) = self.get_server_command(&command, param) {
                                self.editor.add_history_entry(&line);
                                self.last_command = line;
                                return Ok(debugger_command);
                            } else {
                                self.last_command = "".into();
                            }
                        },
                        Err(e) => println!("{}", e)
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
    ) -> Option<DebugCommand> {
        return match command {
            "b" | "breakpoint" => Some(Breakpoint(param)),
            "c" | "continue" => Some(Continue),
            "d" | "delete" => Some(Delete(param)),
            "e" | "exit" => Some(ExitDebugger),
            "f" | "functions" => Some(FunctionList),
            "h" | "?" | "help" => { // only command that doesn't send a message to debugger
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
        This processes a message received from the debug server.
        A message may be generated by the debug server without any a request from the debug client
        Some messages expect the client to respond with a command for the debug server.
    */
    fn process_message(&mut self, message: DebugServerMessage) -> Result<DebugCommand> {
        match message {
            JobCompleted(job) => {
                println!("Job #{} completed by Function #{}", job.job_id, job.function_id);
                if let Ok((Some(output), _)) = job.result {
                    println!("\tOutput value: '{}'", &output);
                }
            }
            PriorToSendingJob(job) => {
                println!("About to send Job #{} to Function #{}", job.job_id, job.function_id);
                println!("\tInputs: {:?}", job.input_set);
            }
            BlockBreakpoint(block) => println!("Block breakpoint: {:?}", block),
            DataBreakpoint(
                source_function_name,
                source_function_id,
                output_route,
                value,
                destination_id,
            destination_name,
            io_name,
                input_number,
            ) => println!(
                "Data breakpoint: Function #{} '{}{}' --{}-> Function #{}:{} '{}'/'{}'",
                source_function_id, source_function_name, output_route, value, destination_id, input_number,
                destination_name, io_name
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
            Deadlock(message) => println!("Deadlock detected{}", message),
            EnteringDebugger => println!(
                "Server is Entering Debugger. Use 'h' or 'help' for help on commands at the prompt"
            ),
            ExitingDebugger => println!("Debugger is exiting"),
            ExecutionStarted => println!("Running flow"),
            ExecutionEnded => println!("Flow has completed"),
            Functions(functions) => Self::function_list(functions),
            SendingValue(source_process_id, value, destination_id, input_number) => println!(
                "Function #{} sending '{}' to {}:{}",
                source_process_id, value, destination_id, input_number
            ),
            DebugServerMessage::Error(error_message) => println!("{}", error_message),
            Message(message) => println!("{}", message),
            Resetting => println!("Resetting state"),
            WaitingForCommand(job_id) => return self.get_user_command(job_id),
            DebugServerMessage::Invalid => println!("Invalid message received from debug server"),
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

    fn function_list(functions: Vec<RuntimeFunction>) {
        println!("Functions List");
        for function in functions {
            println!("\t#{} '{}'", function.id(), function.name());
        }
    }

    /*
       Display information to the user about the current RunState
    */
    fn display_state(run_state: &RunState) {
        println!("{}\n", run_state);

        for id in 0..run_state.num_functions() {
            print!("{}", run_state.get_function(id));
            let function_state = run_state.get_function_state(id);
            println!("\tState: {:?}", function_state);

            if function_state == State::Running {
                println!(
                    "\t\tJob Numbers Running: {:?}",
                    run_state.get_running().get_vec(&id)
                );
            }

            if function_state == State::Blocked {
                for block in run_state.get_blocks() {
                    if block.blocked_id == id {
                        println!("\t\t{:?}", block);
                    }
                }
            }

            // print any blocked or blocking function information
            for block in run_state.get_blocks() {
                if block.blocking_id == id {
                    println!(
                        "\tBlocking #{}:{} <- Blocked #{}",
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

    use flowcore::model::input::Input;
    use flowcore::model::input::InputInitializer::Once;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;
    use flowrlib::run_state::{RunState, State};

    use super::*;

    fn test_function_b_init() -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fB",
            #[cfg(feature = "debugger")]
            "/fB",
            "file://fake/test",
            vec![Input::new("", &Some(Once(json!(1))))],
            1,
            0,
            &[],
            false,
        )
    }

    fn test_function_a_to_b() -> RuntimeFunction {
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
            0,
        );
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fA",
            #[cfg(feature = "debugger")]
            "/fA",
            "file://fake/test",
            vec![Input::new("", &Some(Once(json!(1))))],
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
            state.get_function_state(0),
            "f_a should be in Blocked state"
        );
        assert_eq!(State::Ready, state.get_function_state(1), "f_b should be Ready");
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
        assert_eq!(State::Running, state.get_function_state(1), "f_b should be Running");
        assert_eq!(
            1,
            state.number_jobs_running(),
            "There should be 1 job running"
        );

        CliDebugClient::display_state(&state);
    }
}
