use std::sync::{Arc, Mutex};

use log::error;
use rustyline::Editor;
use rustyline::error::ReadlineError;

use flowcore::errors::*;
use flowcore::model::runtime_function::RuntimeFunction;
use flowrlib::debug_command::BreakpointSpec;
use flowrlib::debug_command::DebugCommand;
use flowrlib::debug_command::DebugCommand::*;
use flowrlib::run_state::{RunState, State};

use crate::cli::client_server::ClientConnection;
use crate::cli::debug_server_message::{DebugServerMessage, DebugServerMessage::*};

const FLOWR_HISTORY_FILENAME: &str = ".flowr_history";

const HELP_STRING: &str = "Debugger commands:
'b' | 'breakpoint' {spec}     - Set a breakpoint using spec:
                                 - on a function by function_id (integer)
                                 - on an output by source_id/output_route ('source_id/' for default output)
                                 - on an input by destination_id:input_number
                                 - on block creation by blocked_process_id->blocking_process_id
'c' | 'continue'              - Continue execution after a breakpoint
'd' | 'delete' {spec} or '*'  - Delete the breakpoint matching {spec} or all with '*'
'e' | 'exit'                  - Stop flow execution and exit debugger
'f' | 'functions'             - Show the list of functions
'h' | 'help' | '?'            - Display this help message
'i' | 'inspect' [n]           - Inspect the overall state, or the function number 'n'
'l' | 'list'                  - List all breakpoints
'm' | 'modify' [name]=[value] - Modify a debugger or runtime variable named 'name' to value 'value'
'q' | 'quit'                  - Stop flow execution and exit debugger
'r' | 'reset' or 'run' {args} - If running already then reset the state, or run the flow with {args}
's' | 'step' [n]              - Step over the next 'n' jobs (default = 1) then break
'v' | 'validate'              - Validate the state of the flow by running a series of checks
";

/*
    A simple CLI (i.e. stdin and stdout) debug client that implements the DebugClient trait
    defined in the flowrlib library.
*/
pub struct CliDebugClient {
    connection: ClientConnection,
    override_args: Arc<Mutex<Vec<String>>>,
    editor: Editor<()>,
    last_command: String,
}

impl CliDebugClient {
    /// Create a new debug client accepting the debug connection
    pub fn new(connection: ClientConnection, override_args: Arc<Mutex<Vec<String>>>) -> Self {
        CliDebugClient {
            connection,
            override_args,
            editor: Editor::<()>::new().expect("Could not create Editor"), // `()` can be used when no completer is required
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
            .send(DebugClientStarting);

        // loop while? and avoid break?
        loop {
            match self.connection.receive() {
                Ok(debug_server_message) => {
                    if let Ok(response) = self.process_server_message(debug_server_message) {
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

    fn parse_command(&self, mut input: String) -> Result<(String, String, Option<Vec<String>>)> {
        input = input.trim().to_string();
        if input.is_empty() && !self.last_command.is_empty() {
            input = self.last_command.clone();
            println!("Repeating last valid command: '{}'", input);
        }

        let parts: Vec<String> = input.split(' ').map(|s| s.to_string()).collect();
        let command = parts[0].to_string();

        if !parts.is_empty() {
            return Ok((input, command, Some(parts[1..].to_vec())));
        };

        Ok((input, command, None))
    }

    fn parse_optional_int(params: Option<Vec<String>>) -> Option<usize> {
        if let Some(param) = params {
            if !param.is_empty() {
                if let Ok(integer) = param[1].parse::<usize>() {
                    return Some(integer);
                }
            }
        }

        None
    }

    fn parse_breakpoint_spec(specs: Option<Vec<String>>) -> Option<BreakpointSpec> {
        if let Some(spec) = specs {
            if !spec.is_empty() {
                if spec[0] == "*" {
                    return Some(BreakpointSpec::All);
                }

                if let Ok(integer) = spec[0].parse::<usize>() {
                    return Some(BreakpointSpec::Numeric(integer));
                }

                if spec[0].contains('/') {
                    // is an output specified
                    let sub_parts: Vec<&str> = spec[0].split('/').collect();
                    if let Ok(source_process_id) = sub_parts[0].parse::<usize>() {
                        return Some(BreakpointSpec::Output((
                                source_process_id,
                                format!("/{}", sub_parts[1]),
                            )));
                    }
                } else if spec[0].contains(':') {
                    // is an input specifier
                    let sub_parts: Vec<&str> = spec[0].split(':').collect();
                    if let (Ok(destination_function_id), Ok(destination_input_number)) = (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
                        return Some(BreakpointSpec::Input((
                                destination_function_id,
                                destination_input_number,
                            )));
                    }
                } else if spec[0].contains("->") {
                    // is a block specifier
                    let sub_parts: Vec<&str> = spec[0].split("->").collect();
                    let source = sub_parts[0].parse::<usize>().ok();
                    let destination = sub_parts[1].parse::<usize>().ok();
                    return Some(BreakpointSpec::Block((source, destination)));
                }
            }
        }

        None
    }

    fn parse_inspect_spec(spec: Option<Vec<String>>) -> Option<DebugCommand> {
        match Self::parse_breakpoint_spec(spec) {
            None => Some(Inspect),
            Some(BreakpointSpec::Numeric(function_id)) => Some(InspectFunction(function_id)),
            Some(BreakpointSpec::Input((function_id, input_number))) => {
                Some(InspectInput(function_id, input_number))
            }
            Some(BreakpointSpec::Output((function_id, sub_route))) => {
                Some(InspectOutput(function_id, sub_route))
            }
            Some(BreakpointSpec::Block((source_function_id, destination_function_id))) => {
                Some(InspectBlock(source_function_id, destination_function_id))
            }
            _ => {
                println!("Unsupported format for 'inspect' command. Use 'h' or 'help' command for help");
                None
            }
        }
    }

    /*
       Wait for the user to input a valid debugger command then return the corresponding response
       that should be sent to the debug server
    */
    fn get_user_command(&mut self, job_number: usize) -> Result<DebugCommand> {
        loop {
            match self.editor.readline(&format!("Job #{}> ", job_number)) {
                Ok(line) => {
                    match self.parse_command(line) {
                        Ok((line, command, params)) => {
                            if let Some(debugger_command) = self.get_server_command(&command, params) {
                                self.editor.add_history_entry(&line);
                                self.last_command = line;
                                return Ok(debugger_command);
                            } else {
                                self.last_command = "".into();
                            }
                        },
                        Err(e) => eprintln!("{}", e)
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Use 'q' or 'quit' to exit the debugger");
                },
                Err(_) => return Ok(ExitDebugger), // Includes CONTROL-D exits
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
        params: Option<Vec<String>>,
    ) -> Option<DebugCommand> {
        return match command {
            "b" | "breakpoint" => Some(Breakpoint(Self::parse_breakpoint_spec(params))),
            "c" | "continue" => Some(Continue),
            "d" | "delete" => Some(Delete(Self::parse_breakpoint_spec(params))),
            "e" | "exit" => Some(ExitDebugger),
            "f" | "functions" => Some(FunctionList),
            "h" | "?" | "help" => { // only command that doesn't send a message to debugger
                Self::help();
                self.editor.add_history_entry(command);
                None
            }
            "i" | "inspect" => Self::parse_inspect_spec(params),
            "l" | "list" => Some(List),
            "m" | "modify" => Some(Modify(params)),
            "q" | "quit" => Some(ExitDebugger),
            "r" | "run" | "reset" => {
                if let Some(mut overrides) = params {
                    if let Ok(mut args) = self.override_args.lock() {
                        args.clear();
                        args.append(&mut overrides);
                    }
                }
                Some(RunReset)
            },
            "s" | "step" => Some(Step(Self::parse_optional_int(params))),
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
    fn process_server_message(&mut self, message: DebugServerMessage) -> Result<DebugCommand> {
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
                "Data breakpoint: Function #{source_function_id} '{source_function_name}{output_route}' \
                --{value}-> Function #{destination_id}:{input_number} '{destination_name}'/'{io_name}'",
            ),
            Panic(message, jobs_created) => {
                println!("Function panicked after {jobs_created} jobs created: {message}");
                return self.get_user_command(jobs_created);
            }
            JobError(job) => {
                println!("Error occurred executing a Job: \n'{job}'");
                return self.get_user_command(job.job_id);
            }
            Deadlock(message) => println!("Deadlock detected {message}"),
            EnteringDebugger => println!(
                "Server is Entering Debugger. Use 'h' or 'help' for help on commands at the prompt"
            ),
            ExitingDebugger => println!("Debugger is exiting"),
            ExecutionStarted => println!("Running flow"),
            ExecutionEnded => println!("Flow has completed"),
            Functions(functions) => Self::function_list(functions),
            SendingValue(source_process_id, value, destination_id, input_number) => println!(
                "Function #{source_process_id} sending '{value}' to {destination_id}:{input_number}",
            ),
            DebugServerMessage::Error(error_message) => println!("{}", error_message),
            Message(message) => println!("{message}"),
            Resetting => println!("Resetting state"),
            WaitingForCommand(job_id) => return self.get_user_command(job_id),
            DebugServerMessage::Invalid => println!("Invalid message received from debug server"),
            FunctionStates((function, state)) => {
                print!("{function}");
                println!("\tState: {:?}", state);
            }
            OverallState(run_state) => Self::display_state(&run_state),
            InputState(input) => println!("{input}"),
            OutputState(output_connections) => {
                if output_connections.is_empty() {
                    println!("No output connections from that sub-route");
                } else {
                    for connection in output_connections {
                        println!("{connection}")
                    }
                }
            }
            BlockState(blocks) => {
                if blocks.is_empty() {
                    println!("No blocks between functions matching the specification were found");
                }
                for block in blocks {
                    println!("{block}");
                }
            }
            FlowUnblockBreakpoint(flow_id) => {
                println!("Flow #{flow_id} was busy and has now gone idle, unblocking senders to functions");
            }
        }

        Ok(Ack)
    }

    fn function_list(functions: Vec<RuntimeFunction>) {
        println!("Functions List");
        for function in functions {
            println!("\t#{} '{}' @ '{}'", function.id(), function.name(), function.route());
        }
        println!("Use 'i n' or 'inspect n' to inspect the function number 'n'");
    }

    /*
       Display information to the user about the current RunState
    */
    fn display_state(run_state: &RunState) {
        println!("{}\n", run_state);

        for id in 0..run_state.num_functions() {
            print!("{}", run_state.get_function(id));
            let function_states = run_state.get_function_states(id);
            println!("\tStates: {:?}", function_states);

            if function_states.contains(&State::Running) {
                println!(
                    "\t\tJob Numbers Running: {:?}",
                    run_state.get_running().get_vec(&id)
                );
            }

            if function_states.contains(&State::Blocked) {
                for block in run_state.get_blocks() {
                    if block.blocked_function_id == id {
                        println!("\t\t{:?}", block);
                    }
                }
            }

            // print any blocked or blocking function information
            for block in run_state.get_blocks() {
                if block.blocking_function_id == id {
                    println!(
                        "\tBlocking #{}:{} <- Blocked #{}",
                        block.blocking_function_id, block.blocking_io_number, block.blocked_function_id
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use url::Url;

    use flowcore::model::input::Input;
    use flowcore::model::input::InputInitializer::Once;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;
    use flowrlib::run_state::RunState;

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
            None,
            true,
        );
        let state = RunState::new(&functions, submission);

        CliDebugClient::display_state(&state);
    }
}
