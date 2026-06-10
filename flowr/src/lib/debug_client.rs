//! [`DebugClient`] — a CLI REPL debug client that connects to a debug server
//! and provides interactive debugging of flow programs.

use log::error;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{DefaultEditor, Editor};

use crate::debug_command::BreakpointSpec;
use crate::debug_command::DebugCommand;
use crate::debug_command::DebugCommand::{
    Ack, Breakpoint, Continue, DebugClientStarting, Delete, ExitDebugger, FunctionList, Inspect,
    InspectFunction, InspectInput, InspectOutput, List, Modify, RunReset, Step, Validate,
};
use crate::debug_command::ProcessTarget;
use crate::debugger::Debugger;
use crate::run_state::RunState;
use flowcore::errors::Result;
use flowcore::model::runtime_function::RuntimeFunction;

use crate::connections::ClientConnection;
use crate::debug_server_message::DebugServerMessage;
use DebugServerMessage::{
    DataBreakpoint, Deadlock, EnteringDebugger, ExecutionEnded, ExecutionStarted, ExitingDebugger,
    FlowUnblockBreakpoint, FunctionStates, Functions, InputState, JobCompleted, JobError, Message,
    OutputState, OverallState, Panic, PriorToSendingJob, Resetting, SendingValue,
    WaitingForCommand,
};

const FLOWDB_HISTORY_FILENAME: &str = ".flowrdb_history";

const HELP_STRING: &str = "Debugger commands:
'b' | 'breakpoint' {spec}     - Set a breakpoint using spec:
                                 - on a function by function_id (integer)
                                 - on job completion by function_id+ (e.g. '3+')
                                 - on an output by source_id/output_route ('source_id/' for default output)
                                 - on an input by destination_id:input_number
                                 - on a function by route path (e.g. '/my-flow/add')
'c' | 'continue'              - Continue execution after a breakpoint
'd' | 'delete' {spec} or '*'  - Delete the breakpoint matching {spec} or all with '*'
'e' | 'exit'                  - Stop flow execution and exit debugger
'f' | 'functions'             - Show the list of functions
'h' | 'help' | '?'            - Display this help message
'i' | 'inspect' [spec]        - Inspect overall state, a function, input, or output
                                 - 'i' with no args shows overall state
                                 - 'i <id>' inspects function by ID
                                 - 'i <id>:<input>' inspects an input
                                 - 'i <id>/<route>' inspects output connections
                                 - 'i ready|waiting|running|completed|blocked' filters by state
                                 - 'i /route/path' inspects function or flow at that route
'l' | 'list'                  - List all breakpoints
'm' | 'modify' [name]=[value] - Modify a debugger or runtime variable named 'name' to value 'value'
'p' | 'processes'             - Show flows and functions in a hierarchical tree
'q' | 'quit'                  - Stop flow execution and exit debugger
'r' | 'reset' or 'run' [target] [args] - No args: reset/run root flow
                                 target: function ID, /route, or name
                                 args: space-separated input values
's' | 'step' [n]              - Step over the next 'n' jobs (default = 1) then break
'v' | 'validate'              - Validate the state of the flow by running a series of checks
";

/// A CLI debug client that uses a REPL to interact with the debug server
pub struct DebugClient {
    connection: ClientConnection,
    editor: Editor<(), DefaultHistory>,
    last_command: String,
}

impl DebugClient {
    /// Create a new debug client with the given connection to a debug server
    ///
    /// # Errors
    /// Returns an error if the terminal editor cannot be created
    pub fn new(connection: ClientConnection) -> Result<Self> {
        Ok(DebugClient {
            connection,
            editor: DefaultEditor::new().map_err(|e| format!("Could not create Editor: {e}"))?,
            last_command: String::new(),
        })
    }

    /// Main debug client loop where events are received, processed and responses sent
    pub fn debug_client_loop(mut self) {
        let _ = self.editor.load_history(FLOWDB_HISTORY_FILENAME);

        let _ = self.connection.send(DebugClientStarting);

        loop {
            match self.connection.receive() {
                Ok(debug_server_message) => {
                    let exiting = matches!(debug_server_message, ExitingDebugger);
                    let response = match self.process_server_message(debug_server_message) {
                        Ok(response) => response,
                        Err(e) => {
                            error!("Error processing server message: {e}");
                            Ack
                        }
                    };
                    let _ = self.connection.send(response);
                    if exiting {
                        break;
                    }
                }
                Err(err) => {
                    error!("Error receiving event from debugger: {err}");
                    break;
                }
            }
        }

        let _ = self.editor.save_history(FLOWDB_HISTORY_FILENAME);
    }

    fn help() {
        println!("{HELP_STRING}");
    }

    fn parse_command(&self, mut input: String) -> Result<(String, String, Option<Vec<String>>)> {
        input = input.trim().to_string();
        if input.is_empty() && !self.last_command.is_empty() {
            input.clone_from(&self.last_command);
            println!("Repeating last valid command: '{input}'");
        }

        let parts: Vec<String> = input.split(' ').map(ToString::to_string).collect();
        let command = parts.first().ok_or("Could not get first part")?.clone();

        if !parts.is_empty() {
            return Ok((
                input,
                command,
                Some(parts.get(1..).ok_or("Could not get parts")?.to_vec()),
            ));
        }

        Ok((input, command, None))
    }

    /// Parse an optional integer from the first element of a parameter list
    #[must_use]
    pub fn parse_optional_int(params: Option<Vec<String>>) -> Option<usize> {
        if let Some(param) = params {
            if !param.is_empty() {
                if let Ok(integer) = param.first()?.parse::<usize>() {
                    return Some(integer);
                }
            }
        }

        None
    }

    /// Parse a breakpoint specification from a parameter list
    #[must_use]
    pub fn parse_breakpoint_spec(specs: Option<Vec<String>>) -> Option<BreakpointSpec> {
        if let Some(spec) = specs {
            if !spec.is_empty() {
                if spec.first()? == "*" {
                    return Some(BreakpointSpec::All);
                }

                if let Some(stripped) = spec.first()?.strip_suffix('+') {
                    if let Ok(process_id) = stripped.parse::<usize>() {
                        return Some(BreakpointSpec::Completed(process_id));
                    }
                }

                if let Ok(integer) = spec.first()?.parse::<usize>() {
                    return Some(BreakpointSpec::Numeric(integer));
                }

                if spec.first()?.starts_with('/') {
                    return Some(BreakpointSpec::Route(spec.first()?.clone()));
                }

                if spec.first()?.contains('/') {
                    let sub_parts: Vec<&str> = spec.first()?.split('/').collect();
                    if let Ok(source_process_id) = sub_parts.first()?.parse::<usize>() {
                        return Some(BreakpointSpec::Output((
                            source_process_id,
                            format!("/{}", sub_parts.get(1)?),
                        )));
                    }
                } else if spec.first()?.contains(':') {
                    let sub_parts: Vec<&str> = spec.first()?.split(':').collect();
                    if let (Ok(destination_function_id), Ok(destination_input_number)) = (
                        sub_parts.first()?.parse::<usize>(),
                        sub_parts.get(1)?.parse::<usize>(),
                    ) {
                        return Some(BreakpointSpec::Input((
                            destination_function_id,
                            destination_input_number,
                        )));
                    }
                }
            }
        }

        None
    }

    /// Valid state keywords for inspect-by-state commands
    pub const STATE_KEYWORDS: &[&str] = &["ready", "waiting", "running", "completed", "blocked"];

    /// Parse an inspect specification into a [`DebugCommand`]
    #[must_use]
    pub fn parse_inspect_spec(spec: Option<Vec<String>>) -> Option<DebugCommand> {
        if let Some(ref params) = spec {
            if let Some(keyword) = params.first() {
                if Self::STATE_KEYWORDS.contains(&keyword.as_str()) {
                    return Some(DebugCommand::InspectState(keyword.clone()));
                }
            }
        }

        match Self::parse_breakpoint_spec(spec) {
            None => Some(Inspect),
            Some(BreakpointSpec::Numeric(function_id)) => Some(InspectFunction(function_id)),
            Some(BreakpointSpec::Input((function_id, input_number))) => {
                Some(InspectInput(function_id, input_number))
            }
            Some(BreakpointSpec::Output((function_id, sub_route))) => {
                Some(InspectOutput(function_id, sub_route))
            }
            Some(BreakpointSpec::Route(route)) => Some(DebugCommand::InspectRoute(route)),
            _ => {
                println!(
                    "Unsupported format for 'inspect' command. Use 'h' or 'help' command for help"
                );
                None
            }
        }
    }

    fn get_user_command(&mut self, job_number: usize) -> Result<DebugCommand> {
        loop {
            match self.editor.readline(&format!("Job #{job_number}> ")) {
                Ok(line) => match self.parse_command(line) {
                    Ok((line, command, params)) => {
                        if let Some(debugger_command) = self.get_server_command(&command, params) {
                            self.editor
                                .add_history_entry(&line)
                                .map_err(|_| "Could not add history line")?;
                            self.last_command = line;
                            return Ok(debugger_command);
                        }
                    }
                    Err(e) => eprintln!("{e}"),
                },
                Err(ReadlineError::Interrupted) => {
                    println!("Use 'q' or 'quit' to exit the debugger");
                }
                Err(_) => return Ok(ExitDebugger),
            }
        }
    }

    fn get_server_command(
        &mut self,
        command: &str,
        params: Option<Vec<String>>,
    ) -> Option<DebugCommand> {
        match command {
            "b" | "breakpoint" => Some(Breakpoint(Self::parse_breakpoint_spec(params))),
            "c" | "continue" => Some(Continue),
            "d" | "delete" => Some(Delete(Self::parse_breakpoint_spec(params))),
            "e" | "exit" | "q" | "quit" => Some(ExitDebugger),
            "f" | "functions" => Some(FunctionList),
            "h" | "?" | "help" => {
                Self::help();
                let _ = self.editor.add_history_entry(command);
                None
            }
            "i" | "inspect" => Self::parse_inspect_spec(params),
            "l" | "list" => Some(List),
            "p" | "processes" => Some(DebugCommand::ProcessList),
            "m" | "modify" => Some(Modify(params)),
            "r" | "run" | "reset" => {
                let parts: Vec<String> = params
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect();
                if parts.is_empty() {
                    Some(RunReset(None, vec![]))
                } else {
                    let Some(target_str) = parts.first() else {
                        return Some(RunReset(None, vec![]));
                    };
                    let args = parts.get(1..).unwrap_or_default().to_vec();
                    let target = if let Ok(id) = target_str.parse::<usize>() {
                        ProcessTarget::Id(id)
                    } else if target_str.starts_with('/') {
                        ProcessTarget::Route(target_str.clone())
                    } else {
                        ProcessTarget::Name(target_str.clone())
                    };
                    Some(RunReset(Some(target), args))
                }
            }
            "s" | "step" => Some(Step(Self::parse_optional_int(params))),
            "v" | "validate" => Some(Validate),
            _ => {
                println!("Unknown debugger command '{command}'\n");
                None
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process_server_message(&mut self, message: DebugServerMessage) -> Result<DebugCommand> {
        match message {
            JobCompleted(job) => {
                println!(
                    "Job #{} completed by Function #{}",
                    job.payload.job_id, job.process_id
                );
                if let Ok((Some(output), _)) = job.result {
                    println!("\tOutput value: '{output}'");
                }
            }
            PriorToSendingJob(job) => {
                println!(
                    "About to send Job #{} to Function #{}",
                    job.payload.job_id, job.process_id
                );
                println!("\tInputs: {:?}", job.payload.input_set);
            }
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
                "Data breakpoint: Function #{source_function_id} \
                '{source_function_name}{output_route}' \
                --{value}-> Function #{destination_id}:{input_number} \
                '{destination_name}'/'{io_name}'",
            ),
            Panic(message, jobs_created) => {
                println!("Function panicked after {jobs_created} jobs created: {message}");
            }
            JobError(job) => {
                println!("Error occurred executing a Job: \n'{job}'");
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
                "Function #{source_process_id} sending '{value}' to \
                {destination_id}:{input_number}",
            ),
            DebugServerMessage::Error(error_message) => println!("{error_message}"),
            Message(message) => println!("{message}"),
            Resetting => println!("Resetting state"),
            WaitingForCommand(job_id) => return self.get_user_command(job_id),
            DebugServerMessage::Invalid => println!("Invalid message received from debug server"),
            FunctionStates((function, state, blockers)) => {
                print!("{function}");
                println!("\tState: {state:?}");
                if !blockers.is_empty() {
                    let blocker_list: Vec<String> =
                        blockers.iter().map(|id| format!("#{id}")).collect();
                    println!("\tWaiting for: {}", blocker_list.join(", "));
                }
            }
            OverallState(run_state) => Self::display_state(&run_state),
            InputState(input) => println!("{input}"),
            OutputState(output_connections) => {
                if output_connections.is_empty() {
                    println!("No output connections from that sub-route");
                } else {
                    for connection in output_connections {
                        println!("{connection}");
                    }
                }
            }
            FlowUnblockBreakpoint(flow_id) => {
                println!(
                    "Flow #{flow_id} was busy and has now gone idle, unblocking senders to \
                    functions"
                );
            }
            DebugServerMessage::BreakpointList(specs) => {
                Self::print_breakpoint_list(&specs);
            }
            DebugServerMessage::ProcessTree(ref state) => {
                println!("{}", Debugger::process_tree(state));
            }
            DebugServerMessage::InspectByState(ref state_name, ref state) => {
                println!("{}", Debugger::inspect_by_state(state, state_name));
            }
            DebugServerMessage::InspectFlow(flow_id, ref state) => {
                println!("{}", Debugger::inspect_flow(state, flow_id));
            }
            DebugServerMessage::InspectFunction(func_id, ref state) => {
                if let Some(func) = state.get_function(func_id) {
                    print!("{func}");
                    println!("\tState: {:?}", state.get_function_states(func_id));
                    for (i, input) in func.inputs().iter().enumerate() {
                        let name = if input.name().is_empty() {
                            format!("input:{i}")
                        } else {
                            format!("input:{i} '{}'", input.name())
                        };
                        if input.is_empty() {
                            println!("\t{name} — empty (waiting)");
                        } else {
                            println!(
                                "\t{name} — {} value(s): {:?}",
                                input.values_available(),
                                input.received_values()
                            );
                        }
                    }
                }
            }
            DebugServerMessage::FlowList(_) => {}
            DebugServerMessage::JobInspect(ref job) => {
                println!("Job #{}", job.payload.job_id);
                println!("  Function: #{} '{}'", job.process_id, job.function_name);
                println!("  Parent Flow: #{}", job.parent_id);
                println!("  Inputs: {:?}", job.payload.input_set);
                println!("  Connections: {}", job.connections.len());
            }
        }

        Ok(Ack)
    }

    fn print_breakpoint_list(specs: &[BreakpointSpec]) {
        if specs.is_empty() {
            println!(
                "No Breakpoints set. Use the 'b' command to set a breakpoint. Use 'h' for help."
            );
            return;
        }
        println!("Active breakpoints:");
        for spec in specs {
            match spec {
                BreakpointSpec::Numeric(id) => println!("\tFunction #{id}"),
                BreakpointSpec::Completed(id) => println!("\tFunction #{id}+ (completion)"),
                BreakpointSpec::Input((id, num)) => println!("\tInput #{id}:{num}"),
                BreakpointSpec::Output((id, route)) => println!("\tOutput #{id}{route}"),
                BreakpointSpec::Route(route) => println!("\tRoute {route}"),
                BreakpointSpec::All => {}
            }
        }
    }

    pub(crate) fn function_list(functions: Vec<RuntimeFunction>) {
        println!("Functions List");
        for function in functions {
            println!(
                "\t#{} '{}' @ '{}'",
                function.id(),
                function.name(),
                function.route()
            );
        }
        println!("Use 'i n' or 'inspect n' to inspect the function number 'n'");
    }

    /// Display information about the current `RunState`
    pub fn display_state(run_state: &RunState) {
        println!("{run_state}\n");

        for id in 0..run_state.num_functions() {
            if let Some(function) = run_state.get_function(id) {
                print!("{function}");
                let function_states = run_state.get_function_states(id);
                println!("\tStates: {function_states:?}");
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::unnecessary_wraps)]
mod test {
    use serde_json::json;

    use crate::run_state::RunState;
    use flowcore::model::flow_manifest::FlowManifest;
    use flowcore::model::input::Input;
    use flowcore::model::input::InputInitializer::Once;
    use flowcore::model::metadata::MetaData;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;

    use super::DebugClient;

    fn test_function_b_init() -> RuntimeFunction {
        RuntimeFunction::new(
            "fB",
            "/fB",
            "file://fake/test",
            vec![Input::new("", 0, false, Some(Once(json!(1))), None)],
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
            false,
            "/fB".to_string(),
            String::default(),
        );
        RuntimeFunction::new(
            "fA",
            "/fA",
            "file://fake/test",
            vec![Input::new("", 0, false, Some(Once(json!(1))), None)],
            0,
            0,
            &[connection_to_f1],
            false,
        )
    }

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".into()],
        }
    }

    fn test_manifest(functions: Vec<RuntimeFunction>) -> FlowManifest {
        let mut manifest = FlowManifest::new(test_meta_data());
        for function in functions {
            manifest.add_function(function);
        }
        manifest
    }

    fn test_submission(functions: Vec<RuntimeFunction>) -> Submission {
        Submission::new(test_manifest(functions), None, None, true)
    }

    #[test]
    fn display_run_state() {
        let f_a = test_function_a_to_b();
        let f_b = test_function_b_init();
        let state = RunState::new(test_submission(vec![f_b, f_a]));

        DebugClient::display_state(&state);
    }

    fn specs(s: &str) -> Option<Vec<String>> {
        Some(vec![s.to_string()])
    }

    #[test]
    fn parse_optional_int_valid() {
        assert_eq!(DebugClient::parse_optional_int(specs("5")), Some(5));
        assert_eq!(DebugClient::parse_optional_int(specs("0")), Some(0));
        assert_eq!(DebugClient::parse_optional_int(specs("100")), Some(100));
    }

    #[test]
    fn parse_optional_int_none_cases() {
        assert_eq!(DebugClient::parse_optional_int(None), None);
        assert_eq!(DebugClient::parse_optional_int(Some(vec![])), None);
        assert_eq!(DebugClient::parse_optional_int(specs("abc")), None);
    }

    #[test]
    fn parse_breakpoint_spec_all() {
        use crate::debug_command::BreakpointSpec;
        assert_eq!(
            DebugClient::parse_breakpoint_spec(specs("*")),
            Some(BreakpointSpec::All)
        );
    }

    #[test]
    fn parse_breakpoint_spec_numeric() {
        use crate::debug_command::BreakpointSpec;
        assert_eq!(
            DebugClient::parse_breakpoint_spec(specs("42")),
            Some(BreakpointSpec::Numeric(42))
        );
    }

    #[test]
    fn parse_breakpoint_spec_output() {
        use crate::debug_command::BreakpointSpec;
        assert_eq!(
            DebugClient::parse_breakpoint_spec(specs("3/result")),
            Some(BreakpointSpec::Output((3, "/result".to_string())))
        );
    }

    #[test]
    fn parse_breakpoint_spec_input() {
        use crate::debug_command::BreakpointSpec;
        assert_eq!(
            DebugClient::parse_breakpoint_spec(specs("5:0")),
            Some(BreakpointSpec::Input((5, 0)))
        );
    }

    #[test]
    fn parse_breakpoint_spec_route() {
        use crate::debug_command::BreakpointSpec;
        assert_eq!(
            DebugClient::parse_breakpoint_spec(specs("/my-flow/add")),
            Some(BreakpointSpec::Route("/my-flow/add".into()))
        );
        assert_eq!(
            DebugClient::parse_breakpoint_spec(specs("/")),
            Some(BreakpointSpec::Route("/".into()))
        );
    }

    #[test]
    fn parse_inspect_spec_route() {
        use crate::debug_command::DebugCommand;
        assert_eq!(
            DebugClient::parse_inspect_spec(specs("/my-flow/add")),
            Some(DebugCommand::InspectRoute("/my-flow/add".into()))
        );
    }

    #[test]
    fn parse_breakpoint_spec_completed() {
        use crate::debug_command::BreakpointSpec;
        assert_eq!(
            DebugClient::parse_breakpoint_spec(specs("3+")),
            Some(BreakpointSpec::Completed(3))
        );
        assert_eq!(
            DebugClient::parse_breakpoint_spec(specs("0+")),
            Some(BreakpointSpec::Completed(0))
        );
    }

    #[test]
    fn parse_breakpoint_spec_none() {
        assert_eq!(DebugClient::parse_breakpoint_spec(None), None);
        assert_eq!(DebugClient::parse_breakpoint_spec(Some(vec![])), None);
    }

    #[test]
    fn parse_breakpoint_spec_invalid() {
        assert_eq!(DebugClient::parse_breakpoint_spec(specs("abc")), None);
    }

    #[test]
    fn parse_inspect_spec_overall() {
        use crate::debug_command::DebugCommand;
        assert_eq!(
            DebugClient::parse_inspect_spec(None),
            Some(DebugCommand::Inspect)
        );
    }

    #[test]
    fn parse_inspect_spec_state_ready() {
        use crate::debug_command::DebugCommand;
        assert_eq!(
            DebugClient::parse_inspect_spec(specs("ready")),
            Some(DebugCommand::InspectState("ready".into()))
        );
    }

    #[test]
    fn parse_inspect_spec_state_blocked() {
        use crate::debug_command::DebugCommand;
        assert_eq!(
            DebugClient::parse_inspect_spec(specs("blocked")),
            Some(DebugCommand::InspectState("blocked".into()))
        );
    }

    #[test]
    fn parse_inspect_spec_function() {
        use crate::debug_command::DebugCommand;
        assert_eq!(
            DebugClient::parse_inspect_spec(specs("3")),
            Some(DebugCommand::InspectFunction(3))
        );
    }

    #[test]
    fn parse_inspect_spec_input() {
        use crate::debug_command::DebugCommand;
        assert_eq!(
            DebugClient::parse_inspect_spec(specs("5:1")),
            Some(DebugCommand::InspectInput(5, 1))
        );
    }

    #[test]
    fn parse_inspect_spec_output() {
        use crate::debug_command::DebugCommand;
        assert_eq!(
            DebugClient::parse_inspect_spec(specs("2/result")),
            Some(DebugCommand::InspectOutput(2, "/result".to_string()))
        );
    }

    #[test]
    fn function_list_display() {
        let f_a = test_function_a_to_b();
        let f_b = test_function_b_init();
        DebugClient::function_list(vec![f_a, f_b]);
    }
}
