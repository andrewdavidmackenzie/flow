use std::collections::HashSet;
use std::fmt;
use std::fmt::Write;

use error_chain::bail;
use log::error;
use serde_json::Value;

use flowcore::errors::Result;
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::runtime_function::RuntimeFunction;

use crate::block::Block;
use crate::debug_command::BreakpointSpec;
use crate::debug_command::DebugCommand;
use crate::debug_command::DebugCommand::{
    Ack, Breakpoint, Continue, DebugClientStarting, Delete, Error, ExitDebugger, Inspect,
    InspectBlock, InspectFunction, InspectInput, InspectOutput, Invalid, List, Modify, RunReset,
    Step, Validate,
};
use crate::debug_command::ProcessTarget;
use crate::debugger_handler::DebuggerHandler;
use crate::job::Job;
use crate::run_state::{RunState, State};

/// Debugger struct contains all the info necessary to conduct a debugging session, storing
/// set breakpoints, connections to the debug client etc
pub struct Debugger<'a> {
    debug_server: &'a mut dyn DebuggerHandler,
    input_breakpoints: HashSet<(usize, usize)>,
    block_breakpoints: HashSet<(usize, usize)>,
    /* blocked_id -> blocking_id */
    output_breakpoints: HashSet<(usize, String)>,
    break_at_job: usize,
    function_breakpoints: HashSet<usize>,
    completed_breakpoints: HashSet<usize>,
    flow_unblock_breakpoints: HashSet<usize>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum BlockType {
    OutputBlocked,
    // Cannot run and send it's Output as a destination Input is full
    UnreadySender, // Has to send output to an empty Input for other process to be able to run
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct BlockerNode {
    function_id: usize,
    block_type: BlockType,
    blockers: Vec<BlockerNode>,
}

#[allow(dead_code)]
impl BlockerNode {
    fn new(process_id: usize, block_type: BlockType) -> Self {
        BlockerNode {
            function_id: process_id,
            block_type,
            blockers: vec![],
        }
    }
}

impl fmt::Display for BlockerNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.block_type {
            BlockType::OutputBlocked => write!(f, " -> #{}", self.function_id),
            BlockType::UnreadySender => write!(f, " <- #{}", self.function_id),
        }
    }
}

impl<'a> Debugger<'a> {
    pub fn new(debug_server: &'a mut dyn DebuggerHandler) -> Self {
        Debugger {
            debug_server,
            input_breakpoints: HashSet::<(usize, usize)>::new(),
            block_breakpoints: HashSet::<(usize, usize)>::new(),
            output_breakpoints: HashSet::<(usize, String)>::new(),
            break_at_job: usize::MAX,
            function_breakpoints: HashSet::<usize>::new(),
            completed_breakpoints: HashSet::<usize>::new(),
            flow_unblock_breakpoints: HashSet::<usize>::new(),
        }
    }

    /// Start the debugger
    pub fn start(&mut self) {
        self.debug_server.start();
    }

    /// Check if there is a breakpoint at this job prior to starting executing it.
    /// Return values are (display next output, reset execution)
    pub fn check_prior_to_job(&mut self, state: &mut RunState, job: &Job) -> Result<(bool, bool)> {
        if self.break_at_job == job.payload.job_id
            || self.function_breakpoints.contains(&job.process_id)
        {
            self.debug_server.job_breakpoint(
                job,
                state
                    .get_function(job.process_id)
                    .ok_or("Could not get function")?,
                state.get_function_states(job.process_id),
            );
            return self.wait_for_command(state);
        }

        Ok((false, false))
    }

    /// Called from flowrlib during execution when it is about to create a block on one function
    /// due to not being able to send outputs to another function.
    ///
    /// This allows the debugger to check if we have a breakpoint set on that block. If we do
    /// then enter the debugger client and wait for a command.
    #[allow(dead_code)]
    pub fn check_on_block_creation(
        &mut self,
        state: &mut RunState,
        block: &Block,
    ) -> Result<(bool, bool)> {
        if self
            .block_breakpoints
            .contains(&(block.blocked_function_id, block.blocking_function_id))
        {
            self.debug_server.block_breakpoint(block);
            return self.wait_for_command(state);
        }

        Ok((false, false))
    }

    /// Called from flowrlib runtime prior to sending a value to another function,to see if there
    /// is a breakpoint on that send.
    ///
    /// If there is, then enter the debug client and wait for a command.
    pub fn check_prior_to_send(
        &mut self,
        state: &mut RunState,
        source_function_id: usize,
        output_route: &str,
        value: &Value,
        destination_id: usize,
        input_number: usize,
    ) -> Result<(bool, bool)> {
        if self
            .output_breakpoints
            .contains(&(source_function_id, output_route.to_string()))
            || self
                .input_breakpoints
                .contains(&(destination_id, input_number))
        {
            let source_function = state
                .get_function(source_function_id)
                .ok_or("Could not get function")?;
            let destination_function = state
                .get_function(destination_id)
                .ok_or("Could not get function")?;
            let io_name = destination_function
                .input(input_number)
                .ok_or("Could not get input")?
                .name();

            self.debug_server.send_breakpoint(
                source_function.name(),
                source_function_id,
                output_route,
                value,
                destination_id,
                destination_function.name(),
                io_name,
                input_number,
            );
            return self.wait_for_command(state);
        }

        Ok((false, false))
    }

    /// Called from flowrlib runtime prior to unblocking a flow to see if there is a breakpoint
    /// set on that event
    ///
    /// If there is, then enter the debug client and wait for a command.
    pub fn check_prior_to_flow_unblock(
        &mut self,
        state: &mut RunState,
        flow_being_unblocked_id: usize,
    ) -> Result<(bool, bool)> {
        if self
            .flow_unblock_breakpoints
            .contains(&flow_being_unblocked_id)
        {
            self.debug_server
                .flow_unblock_breakpoint(flow_being_unblocked_id);
            return self.wait_for_command(state);
        }

        Ok((false, false))
    }

    /// An error occurred while executing a flow. Let the debug client know, enter the client
    /// and wait for a user command.
    ///
    /// This is useful for debugging a flow that has an error. Without setting any explicit
    /// breakpoint it will enter the debugger on an error and let the user inspect the flow's
    /// state etc.
    pub fn job_error(&mut self, state: &mut RunState, job: &Job) -> Result<(bool, bool)> {
        self.debug_server.job_error(job);
        self.wait_for_command(state)
    }

    /// Called from the flowrlib coordinator to inform the debug client that a job has completed
    /// Return values are (display next output, reset execution)
    pub fn job_done(&mut self, state: &mut RunState, job: &Job) -> (bool, bool) {
        if job.result.is_err() {
            if state.submission.debug_enabled {
                let _ = self.job_error(state, job);
            }
        } else {
            self.debug_server.job_completed(job);
            if self.completed_breakpoints.contains(&job.process_id) {
                if let Some(function) = state.get_function(job.process_id) {
                    self.debug_server.job_breakpoint(
                        job,
                        function,
                        state.get_function_states(job.process_id),
                    );
                    if let Ok(result) = self.wait_for_command(state) {
                        return result;
                    }
                }
            }
        }
        (false, false)
    }

    /// An error occurred while executing a flow. Let the debug client know, enter the client
    /// and wait for a user command.
    ///
    /// This is useful for debugging a flow that has an error. Without setting any explicit
    /// breakpoint it will enter the debugger on an error and let the user inspect the flow's
    /// state etc.
    pub fn error(&mut self, state: &mut RunState, error_message: String) -> Result<(bool, bool)> {
        self.debug_server.panic(state, error_message);
        self.wait_for_command(state)
    }

    /// Execution of the flow ended, report it and wait for command
    /// Return values are (display next output, reset execution)
    pub fn execution_ended(&mut self, state: &mut RunState) -> Result<(bool, bool)> {
        self.debug_server.execution_ended();
        self.wait_for_command(state)
    }

    /// The execution flow has entered the debugged based on some event.
    ///
    /// Now wait for and process commands from the `DebugClient`
    /// - execute and respond immediately those that require it
    /// - some commands will cause the command loop to exit.
    ///
    /// When exiting return a set of booleans for the Coordinator to determine what to do:
    /// (display next output, reset execution, `exit_debugger`)
    #[allow(clippy::too_many_lines)]
    pub fn wait_for_command(&mut self, state: &mut RunState) -> Result<(bool, bool)> {
        loop {
            match self.debug_server.get_command(state) {
                // *************************      The following are commands that send a response
                Ok(Breakpoint(param)) => {
                    let result = self.add_breakpoint(state, param);
                    let message = result.unwrap_or_else(|e| e.to_string());
                    self.debug_server.message(message);
                }
                Ok(Delete(param)) => {
                    let result = self.delete_breakpoint(state, param);
                    let message = result.unwrap_or_else(|e| e.to_string());
                    self.debug_server.message(message);
                }
                Ok(Validate) => {
                    let message = Self::validate(state);
                    self.debug_server.message(message);
                }
                Ok(List) => {
                    let specs = self.collect_breakpoint_specs();
                    self.debug_server.breakpoint_list(specs);
                }
                Ok(DebugCommand::FunctionList) => {
                    let mut functions: Vec<RuntimeFunction> =
                        state.get_functions().values().cloned().collect();
                    functions.sort_by_key(RuntimeFunction::id);
                    self.debug_server.function_list(&functions);
                }
                Ok(DebugCommand::ProcessList) => {
                    self.debug_server.process_tree(state);
                }
                Ok(Inspect) => self.debug_server.run_state(state),
                Ok(DebugCommand::InspectState(ref state_name)) => {
                    self.debug_server.inspect_by_state(state_name, state);
                }
                Ok(DebugCommand::InspectRoute(ref route)) => {
                    if let Some(func_id) = Self::find_by_route(state, route) {
                        if state.submission.manifest.flows().contains_key(&func_id) {
                            self.debug_server.inspect_flow(func_id, state);
                        } else if state.get_function(func_id).is_some() {
                            self.debug_server.function_states(
                                state
                                    .get_function(func_id)
                                    .ok_or("Could not get function")?
                                    .clone(),
                                state.get_function_states(func_id),
                            );
                        }
                    } else {
                        self.debug_server.debugger_error(format!(
                            "No function or flow found at route '{route}'"
                        ));
                    }
                }
                Ok(InspectFunction(function_id)) => {
                    if state.get_function(function_id).is_some() {
                        self.debug_server.function_states(
                            state
                                .get_function(function_id)
                                .ok_or("Could not get function")?
                                .clone(),
                            state.get_function_states(function_id),
                        );
                    } else if state.submission.manifest.flows().contains_key(&function_id) {
                        self.debug_server.inspect_flow(function_id, state);
                    } else {
                        self.debug_server
                            .debugger_error(format!("No function or flow with id = {function_id}"));
                    }
                }
                Ok(InspectInput(function_id, input_number)) => {
                    if state.get_function(function_id).is_some() {
                        let function = state
                            .get_function(function_id)
                            .ok_or("Could not get function")?;

                        if input_number < function.inputs().len() {
                            self.debug_server.input(
                                function
                                    .input(input_number)
                                    .ok_or("Could not get input")?
                                    .clone(),
                            );
                        } else {
                            self.debug_server.debugger_error(format!(
                                "Function #{function_id} has no input number {input_number}"
                            ));
                        }
                    } else {
                        self.debug_server
                            .debugger_error(format!("No function with id = {function_id}"));
                    }
                }
                Ok(InspectOutput(function_id, sub_route)) => {
                    if state.get_function(function_id).is_some() {
                        let function = state
                            .get_function(function_id)
                            .ok_or("Could not get function")?;

                        let mut output_connections = vec![];

                        for output_connection in function.get_output_connections() {
                            match &output_connection.source {
                                Output(source_route) => {
                                    if *source_route == sub_route {
                                        output_connections.push(output_connection.clone());
                                    }
                                }
                                // add list of connections from an input to job if path "" is specified
                                Input(_) => {
                                    if sub_route.is_empty() {
                                        output_connections.push(output_connection.clone());
                                    }
                                }
                            }
                        }
                        self.debug_server.outputs(output_connections);
                    } else {
                        self.debug_server
                            .debugger_error(format!("No function with id = {function_id}"));
                    }
                }
                Ok(InspectBlock(from_function_id, to_function_id)) => {
                    let blocks = Self::inspect_blocks(state, from_function_id, to_function_id);
                    self.debug_server.blocks(blocks);
                }
                Ok(Modify(specs)) => self.modify_variables(state, &specs),
                Ok(DebugClientStarting) => {
                    // TODO remove
                    error!("Unexpected message 'DebugClientStarting' after started");
                }
                Ok(Error(_) | Ack | Invalid) => {}
                Err(e) => error!("Error in Debug server getting command; {e}"),

                // ************************** The following commands may exit the command loop
                Ok(Continue) => {
                    if state.get_number_of_jobs_created() > 0 {
                        return Ok((false, false));
                    }
                }
                Ok(RunReset(None, _)) => {
                    return if state.get_number_of_jobs_created() > 0 {
                        self.reset();
                        self.debug_server.debugger_resetting();
                        Ok((false, true))
                    } else {
                        self.debug_server.execution_starting();
                        Ok((false, false))
                    };
                }
                Ok(RunReset(Some(target), args)) => {
                    if state.get_number_of_jobs_created() > 0 {
                        self.reset();
                        state.reset();
                    }
                    match Self::resolve_target(state, &target) {
                        Ok(process_id) => match self.run_process(state, process_id, &args) {
                            Ok(()) => {
                                self.debug_server.execution_starting();
                                return Ok((false, false));
                            }
                            Err(e) => self.debug_server.debugger_error(e.to_string()),
                        },
                        Err(e) => self.debug_server.debugger_error(e.to_string()),
                    }
                }
                Ok(Step(param)) => {
                    self.step(state, param);
                    return Ok((true, false));
                }
                Ok(ExitDebugger) => {
                    self.debug_server.debugger_exiting();
                    bail!("Debugger Exit");
                }
            }
        }
    }

    /*
       Find current blocks that match the spec.
       Blocking has been removed, so this always returns an empty list.
    */
    #[allow(dead_code)]
    fn inspect_blocks(
        _run_state: &RunState,
        _from: Option<usize>,
        _to: Option<usize>,
    ) -> Vec<Block> {
        vec![]
    }

    /****************************** Implementations of Debugger Commands *************************/

    /*
       Add a breakpoint to the debugger according to the Optional `Param`
    */
    fn add_breakpoint(
        &mut self,
        state: &RunState,
        param: Option<BreakpointSpec>,
    ) -> Result<String> {
        match param {
            None => bail!("'break' command must specify a breakpoint\n"),
            Some(BreakpointSpec::All) => {
                bail!("To break on every Function, you can just single step using 's' command\n")
            }
            Some(BreakpointSpec::Numeric(process_id)) => {
                if state.get_function(process_id).is_none() {
                    bail!("There is no Function with id '{process_id}' to set a breakpoint on");
                }

                self.function_breakpoints.insert(process_id);
                let function = state
                    .get_function(process_id)
                    .ok_or("Could not get function")?;
                Ok(format!(
                    "Breakpoint set on Function #{} ({}) @ '{}'",
                    process_id,
                    function.name(),
                    function.route()
                ))
            }
            Some(BreakpointSpec::Input((destination_id, input_number))) => {
                if state.get_function(destination_id).is_none() {
                    bail!("There is no Function #{destination_id} to set a breakpoint on");
                }

                let function = state
                    .get_function(destination_id)
                    .ok_or("Could not get function")?;

                if input_number >= function.inputs().len() {
                    bail!("There is no Input :{input_number} on function #{destination_id}");
                }
                let io_name = function
                    .input(input_number)
                    .ok_or("Could not get input")?
                    .name();
                self.input_breakpoints
                    .insert((destination_id, input_number));
                Ok(format!(
                    "Data breakpoint set on Function #{destination_id}:{input_number} '{}' receiving data on input '{io_name}'",
                    function.name()))
            }
            Some(BreakpointSpec::Block((Some(blocked_id), Some(blocking_id)))) => {
                if state.get_function(blocked_id).is_none() {
                    bail!("There is no Function #{blocked_id} to set a Block breakpoint on");
                }

                if state.get_function(blocking_id).is_none() {
                    bail!("There is no Function #{blocking_id} to set a Block breakpoint on");
                }

                self.block_breakpoints.insert((blocked_id, blocking_id));
                Ok(format!(
                    "Block breakpoint set on Function #{blocked_id} being blocked by Function #{blocking_id}"))
            }
            Some(BreakpointSpec::Block(_)) => {
                bail!("Invalid format to set a breakpoint on a block\n")
            }
            Some(BreakpointSpec::Output((source_id, source_output_route))) => {
                if state.get_function(source_id).is_none() {
                    bail!("There is no Function #{source_id} to set a Output breakpoint on");
                }

                self.output_breakpoints
                    .insert((source_id, source_output_route.clone()));
                Ok(format!(
                    "Data breakpoint set on Function #{source_id} sending data via output: '{source_output_route}'"
                ))
            }
            Some(BreakpointSpec::Completed(process_id)) => {
                if state.get_function(process_id).is_none() {
                    bail!(format!(
                        "There is no Function with id '{process_id}' to set a completion breakpoint on"
                    ));
                }

                self.completed_breakpoints.insert(process_id);
                let function = state
                    .get_function(process_id)
                    .ok_or("Could not get function")?;
                Ok(format!(
                    "Completion breakpoint set on Function #{} ({}) @ '{}'",
                    process_id,
                    function.name(),
                    function.route()
                ))
            }
            Some(BreakpointSpec::Route(route)) => {
                if let Some(process_id) = Self::find_by_route(state, &route) {
                    self.function_breakpoints.insert(process_id);
                    let function = state
                        .get_function(process_id)
                        .ok_or("Could not get function")?;
                    Ok(format!(
                        "Breakpoint set on Function #{} ({}) @ '{}'",
                        process_id,
                        function.name(),
                        function.route()
                    ))
                } else {
                    bail!(format!("No function or flow found at route '{route}'"))
                }
            }
        }
    }

    /*
       Delete debugger breakpoints related to Jobs or Blocks, etc. according to the Spec.
    */
    fn delete_breakpoint(
        &mut self,
        state: &RunState,
        param: Option<BreakpointSpec>,
    ) -> Result<String> {
        match param {
            None => bail!("No process id specified\n"),
            Some(BreakpointSpec::All) => {
                self.output_breakpoints.clear();
                self.input_breakpoints.clear();
                self.function_breakpoints.clear();
                self.completed_breakpoints.clear();
                Ok("Deleted all breakpoints\n".into())
            }
            Some(BreakpointSpec::Numeric(process_number)) => {
                if state.get_function(process_number).is_none() {
                    bail!("There is no Function with id '{process_number}' to delete a breakpoint from");
                }

                if self.function_breakpoints.remove(&process_number) {
                    Ok(format!(
                        "Breakpoint on process #{process_number} was deleted"
                    ))
                } else {
                    bail!("No breakpoint number '{}' exists\n")
                }
            }
            Some(BreakpointSpec::Input((destination_id, input_number))) => {
                if state.get_function(destination_id).is_none() {
                    bail!("There is no Function #{destination_id} to delete a breakpoint from");
                }

                let function = state
                    .get_function(destination_id)
                    .ok_or("Could not get function")?;

                if input_number >= function.inputs().len() {
                    bail!("There is no Input :{input_number} on function #{destination_id}");
                }

                self.input_breakpoints
                    .remove(&(destination_id, input_number));
                Ok("Inputs breakpoint removed\n".into())
            }
            Some(BreakpointSpec::Block((Some(blocked_id), Some(blocking_id)))) => {
                if state.get_function(blocked_id).is_none() {
                    bail!("There is no Function #{blocked_id} to delete a Block breakpoint from");
                }

                if state.get_function(blocking_id).is_none() {
                    bail!("There is no Function #{blocking_id} to delete a Block breakpoint from");
                }

                self.input_breakpoints.remove(&(blocked_id, blocking_id));
                Ok("Inputs breakpoint removed\n".into())
            }
            Some(BreakpointSpec::Block(_)) => bail!("Invalid format to remove breakpoint\n"),
            Some(BreakpointSpec::Output((source_id, source_output_route))) => {
                if state.get_function(source_id).is_none() {
                    bail!("There is no Function #{source_id} to delete a Output breakpoint from");
                }

                self.output_breakpoints
                    .remove(&(source_id, source_output_route));
                Ok("Output breakpoint removed\n".into())
            }
            Some(BreakpointSpec::Completed(process_id)) => {
                if state.get_function(process_id).is_none() {
                    bail!(format!(
                        "There is no Function with id '{process_id}' to delete a completion breakpoint from"
                    ));
                }

                if self.completed_breakpoints.remove(&process_id) {
                    Ok(format!(
                        "Completion breakpoint on Function #{process_id} was deleted"
                    ))
                } else {
                    bail!("No completion breakpoint on Function #{process_id} exists\n")
                }
            }
            Some(BreakpointSpec::Route(route)) => {
                if let Some(process_id) = Self::find_by_route(state, &route) {
                    if self.function_breakpoints.remove(&process_id) {
                        Ok(format!(
                            "Breakpoint on '{route}' (Function #{process_id}) was deleted"
                        ))
                    } else {
                        bail!(format!("No breakpoint on '{route}' exists"))
                    }
                } else {
                    bail!(format!("No function or flow found at route '{route}'"))
                }
            }
        }
    }

    /*
       List all debugger breakpoints of all types.
       // TODO make structs not a string
    */
    fn collect_breakpoint_specs(&self) -> Vec<BreakpointSpec> {
        let mut specs = Vec::new();
        for &id in &self.function_breakpoints {
            specs.push(BreakpointSpec::Numeric(id));
        }
        for &id in &self.completed_breakpoints {
            specs.push(BreakpointSpec::Completed(id));
        }
        for &(func_id, input_num) in &self.input_breakpoints {
            specs.push(BreakpointSpec::Input((func_id, input_num)));
        }
        for (func_id, route) in &self.output_breakpoints {
            specs.push(BreakpointSpec::Output((*func_id, route.clone())));
        }
        for &(blocked, blocking) in &self.block_breakpoints {
            specs.push(BreakpointSpec::Block((Some(blocked), Some(blocking))));
        }
        specs
    }

    fn find_by_route(state: &RunState, route: &str) -> Option<usize> {
        state
            .get_functions()
            .values()
            .find(|f| f.route() == route)
            .map(RuntimeFunction::id)
    }

    fn resolve_target(state: &RunState, target: &ProcessTarget) -> Result<usize> {
        match target {
            ProcessTarget::Id(id) => {
                if state.get_function(*id).is_some()
                    || state.submission.manifest.flows().contains_key(id)
                {
                    Ok(*id)
                } else {
                    bail!("No process found matching '#{id}'")
                }
            }
            ProcessTarget::Route(route) => Self::find_by_route(state, route)
                .ok_or_else(|| format!("No process found matching '{route}'").into()),
            ProcessTarget::Name(name) => {
                let matches: Vec<usize> = state
                    .get_functions()
                    .values()
                    .filter(|f| f.name() == name)
                    .map(RuntimeFunction::id)
                    .collect();
                match matches.as_slice() {
                    [] => bail!("No process found matching '{name}'"),
                    [id] => Ok(*id),
                    _ => {
                        let mut msg =
                            format!("Multiple processes match '{name}'. Use ID or route:\n");
                        for id in &matches {
                            if let Some(f) = state.get_function(*id) {
                                use std::fmt::Write;
                                let _ = writeln!(msg, "  {}", Self::entity_long(f, false));
                            }
                        }
                        bail!(msg)
                    }
                }
            }
        }
    }

    #[allow(clippy::unused_self)]
    fn run_process(
        &mut self,
        state: &mut RunState,
        process_id: usize,
        inline_args: &[String],
    ) -> Result<()> {
        if state.submission.manifest.flows().contains_key(&process_id) {
            bail!(
                "Sub-flow execution is not yet implemented. \
                 Use a function ID or route to run individual functions."
            );
        }

        let function = state
            .get_function(process_id)
            .ok_or("Could not get function")?;
        let num_inputs = function.inputs().len();
        let parent_id = function.get_parent_id();

        if inline_args.len() > num_inputs {
            bail!(
                "Process has {num_inputs} inputs but {} values were provided",
                inline_args.len()
            );
        }

        let mut input_info: Vec<(String, bool, Option<String>)> = Vec::new();
        for (i, input) in function.inputs().iter().enumerate() {
            let name = if input.name().is_empty() {
                format!("input_{i}")
            } else {
                input.name().to_string()
            };
            let generic = input.is_generic();
            let default = if i < inline_args.len() {
                Some(inline_args.get(i).ok_or("Could not get arg")?.clone())
            } else {
                input
                    .initializer()
                    .as_ref()
                    .map(|init| init.get_value().to_string())
                    .or_else(|| {
                        input
                            .flow_initializer()
                            .as_ref()
                            .map(|init| init.get_value().to_string())
                    })
            };
            input_info.push((name, generic, default));
        }

        let missing: Vec<&str> = input_info
            .iter()
            .filter(|(_, _, default)| default.is_none())
            .map(|(name, _, _)| name.as_str())
            .collect();
        if !missing.is_empty() {
            bail!(
                "Missing values for inputs: {}. Supply them as arguments: r <target> <val1> <val2> ...",
                missing.join(", ")
            );
        }

        let mut coerced_values = Vec::new();
        for (_name, _generic, default) in &input_info {
            let raw = default.as_ref().ok_or("Missing input value")?;
            coerced_values.push(crate::coerce_value::coerce_generic(raw));
        }

        let func = state.get_mut(process_id).ok_or("Could not get function")?;
        for (i, value) in coerced_values.into_iter().enumerate() {
            func.send(i, value)?;
        }

        state.create_jobs(process_id, parent_id)?;
        Ok(())
    }

    fn entity_long(func: &RuntimeFunction, is_flow: bool) -> String {
        let entity = if is_flow { "Flow" } else { "Function" };
        format!(
            "{entity} #{} '{}' @ {}",
            func.id(),
            func.name(),
            func.route()
        )
    }

    pub fn inspect_flow(state: &RunState, flow_id: usize) -> String {
        let mut response = String::new();
        let _ = writeln!(response, "Flow #{flow_id}");
        if let Some(flow_info) = state.submission.manifest.flows().get(&flow_id) {
            if let Some(parent) = flow_info.parent_id {
                let _ = writeln!(response, "Parent: Flow #{parent}");
            } else {
                let _ = writeln!(response, "Parent: (none — this is the root flow)");
            }
            if !flow_info.sub_flow_ids.is_empty() {
                let subs: Vec<String> = flow_info
                    .sub_flow_ids
                    .iter()
                    .map(|id| format!("Flow #{id}"))
                    .collect();
                let _ = writeln!(response, "Sub-flows: {}", subs.join(", "));
            }
        }
        let mut functions: Vec<_> = state
            .get_functions()
            .values()
            .filter(|f| {
                f.get_parent_id() == flow_id
                    && f.id() != flow_id
                    && !state.submission.manifest.flows().contains_key(&f.id())
            })
            .collect();
        functions.sort_by_key(|f| f.id());
        if !functions.is_empty() {
            let _ = writeln!(response, "Functions:");
            for func in functions {
                let states = state.get_function_states(func.id());
                let _ = writeln!(response, "  {} {states:?}", Self::entity_long(func, false));
            }
        }
        response
    }

    #[cfg(test)]
    fn inspect_by_route(state: &RunState, route: &str) -> String {
        let Some(func_id) = Self::find_by_route(state, route) else {
            return format!("No function or flow found at route '{route}'");
        };

        let is_flow = state.submission.manifest.flows().contains_key(&func_id);

        let mut response = String::new();

        if is_flow {
            let function = state.get_functions().get(&func_id).cloned();
            if let Some(ref func) = function {
                let _ = writeln!(response, "{}", Self::entity_long(func, true));
            }
            let _ = writeln!(response, "Children:");
            let mut children: Vec<_> = state
                .get_functions()
                .values()
                .filter(|f| f.get_parent_id() == func_id && f.id() != func_id)
                .collect();
            children.sort_by_key(|f| f.id());
            if children.is_empty() {
                let _ = writeln!(response, "  (none)");
            } else {
                for child in children {
                    let states = state.get_function_states(child.id());
                    let _ = writeln!(
                        response,
                        "  {} {:?}",
                        Self::entity_long(child, false),
                        states
                    );
                }
            }
        } else if let Some(function) = state.get_function(func_id) {
            let _ = writeln!(response, "{}", Self::entity_long(function, false));
            let states = state.get_function_states(func_id);
            let _ = writeln!(response, "  State: {states:?}");
        }

        response
    }

    pub fn inspect_by_state(state: &RunState, state_name: &str) -> String {
        let target_state = match state_name {
            "ready" => Some(State::Ready),
            "waiting" => Some(State::Waiting),
            "running" => Some(State::Running),
            "completed" => Some(State::Completed),
            "blocked" => None,
            _ => {
                return format!(
                "Unknown state '{state_name}'. Use: ready, waiting, running, completed, blocked"
            )
            }
        };

        let mut response = String::new();
        let functions = state.get_functions();
        let mut sorted: Vec<_> = functions.values().collect();
        sorted.sort_by_key(|f| f.id());
        let mut count = 0;

        if state_name == "blocked" {
            let _ = writeln!(response, "Blocked functions:");
            for func in &sorted {
                let states = state.get_function_states(func.id());
                if states.contains(&State::Waiting) {
                    if let Ok(blockers) = state.get_input_blockers(func.id()) {
                        if !blockers.is_empty() {
                            count += 1;
                            let _ = writeln!(
                                response,
                                "  {} — blocked by: {:?}",
                                Self::entity_long(func, false),
                                blockers
                            );
                        }
                    }
                }
            }
        } else if let Some(ref target) = target_state {
            let _ = writeln!(response, "Functions in '{state_name}' state:");
            for func in &sorted {
                let states = state.get_function_states(func.id());
                if states.contains(target) {
                    count += 1;
                    let _ = write!(response, "  {}", Self::entity_long(func, false));
                    if state_name == "running" {
                        let running_jobs: Vec<_> = state
                            .get_running()
                            .iter()
                            .filter(|(_, job)| job.process_id == func.id())
                            .map(|(job_id, _)| format!("Job #{job_id}"))
                            .collect();
                        if !running_jobs.is_empty() {
                            let _ = write!(response, " ({})", running_jobs.join(", "));
                        }
                    }
                    let _ = writeln!(response);
                }
            }
        }

        if count == 0 {
            let _ = writeln!(response, "  (none)");
        }

        response
    }

    pub fn process_tree(state: &RunState) -> String {
        use std::collections::BTreeMap;

        let functions = state.get_functions();
        let mut by_parent: BTreeMap<usize, Vec<&RuntimeFunction>> = BTreeMap::new();
        for func in functions.values() {
            let parent = func.get_parent_id();
            by_parent.entry(parent).or_default().push(func);
        }
        for children in by_parent.values_mut() {
            children.sort_by_key(|f| f.id());
        }

        let mut response = String::new();

        let root_parents: Vec<usize> = by_parent
            .keys()
            .filter(|pid| !functions.contains_key(pid))
            .copied()
            .collect();

        for root_id in root_parents {
            Self::print_tree(&by_parent, functions, state, root_id, 0, &mut response);
        }

        let mut self_roots: Vec<_> = functions
            .values()
            .filter(|func| func.get_parent_id() == func.id())
            .collect();
        self_roots.sort_by_key(|func| func.id());
        for func in self_roots {
            Self::print_tree(&by_parent, functions, state, func.id(), 0, &mut response);
        }

        response
    }

    fn print_tree(
        by_parent: &std::collections::BTreeMap<usize, Vec<&RuntimeFunction>>,
        all_functions: &std::collections::HashMap<usize, RuntimeFunction>,
        state: &RunState,
        parent_id: usize,
        depth: usize,
        response: &mut String,
    ) {
        let indent = "  ".repeat(depth);
        if let Some(parent) = all_functions.get(&parent_id) {
            let is_flow = state.submission.manifest.flows().contains_key(&parent_id);
            let states = state.get_function_states(parent.id());
            let _ = writeln!(
                response,
                "{indent}{} {:?}",
                Self::entity_long(parent, is_flow),
                states
            );
        } else {
            let _ = writeln!(response, "{indent}Flow #{parent_id}");
        }

        if let Some(children) = by_parent.get(&parent_id) {
            for child in children {
                if child.id() == parent_id {
                    continue;
                }
                if by_parent.contains_key(&child.id()) {
                    Self::print_tree(
                        by_parent,
                        all_functions,
                        state,
                        child.id(),
                        depth + 1,
                        response,
                    );
                } else {
                    let child_indent = "  ".repeat(depth + 1);
                    let states = state.get_function_states(child.id());
                    let _ = writeln!(
                        response,
                        "{child_indent}{} {:?}",
                        Self::entity_long(child, false),
                        states
                    );
                }
            }
        }
    }

    fn validate(_state: &RunState) -> String {
        let mut response = String::new();

        response.push_str("Deadlock check: ");
        response.push_str(&Self::deadlock_check());

        response
    }

    // Get ready to start execution (and debugging) from scratch at the start of the flow
    fn reset(&mut self) {
        // Leave all the breakpoints untouched for the repeat run
        self.break_at_job = usize::MAX;
    }

    // Parse a series of specs to modify a state value
    #[allow(clippy::ref_option)]
    fn modify_variables(&mut self, state: &mut RunState, specs: &Option<Vec<String>>) {
        match specs.as_deref() {
            None | Some([]) => self.debug_server.message(
                "State variables that can be modified are:\
            \n'jobs' - maximum number of parallel jobs (integer) or 0 for no limit"
                    .to_string(),
            ),
            Some(specs) => {
                for spec in specs {
                    let parts: Vec<&str> = spec.trim().split('=').collect();
                    if parts.len() < 2 {
                        self.debug_server.message(format!(
                            "Invalid modify command for state variables: '{spec}'"
                        ));
                        return;
                    }

                    match parts.first() {
                        Some(&"jobs") => {
                            if let Some(var) = parts.get(1) {
                                if let Ok(value) = var.parse::<usize>() {
                                    if value == 0 {
                                        state.submission.max_parallel_jobs = None;
                                    } else {
                                        state.submission.max_parallel_jobs = Some(value);
                                    }
                                    self.debug_server
                                        .message(format!("State variable 'jobs' set to {var}"));
                                } else {
                                    self.debug_server.message(format!(
                                        "Invalid value '{var}' for variable 'jobs'"
                                    ));
                                }
                            }
                        }
                        _ => self
                            .debug_server
                            .message("Unknown state variable".to_string()),
                    }
                }
            }
        }
    }

    /*
     Take one step (execute one more job) in the flow. Do this by setting a breakpoint at the
     next job execution and then returning - flow execution will continue until breakpoint fires
    */
    fn step(&mut self, state: &RunState, steps: Option<usize>) {
        match steps {
            None => {
                self.break_at_job = state.get_number_of_jobs_created() + 1;
            }
            Some(steps) => {
                if steps > 1 {
                    self.break_at_job = state.get_number_of_jobs_created() + steps;
                } else {
                    self.debug_server
                        .debugger_error("Number of jobs to 'step' must be greater than 0\n".into());
                }
            }
        }
    }

    /*
        Return a vector of all the processes preventing process_id from running, which can be:
        - other process has input full and hence is blocking running of this process
        - other process is the only process that sends to an empty input of this process
    */
    #[allow(dead_code)]
    fn find_blockers(state: &RunState, process_id: usize) -> Result<Vec<BlockerNode>> {
        let input_blockers: Vec<BlockerNode> = state
            .get_input_blockers(process_id)?
            .iter()
            .map(|id| BlockerNode::new(*id, BlockType::UnreadySender))
            .collect();

        Ok(input_blockers)
    }

    /*
        Traverse the tree of processes blocking this process from running, either because:
        - this process wants to send to the other, but the input is full
        - this process needs an input from the other

        Return true if a loop was detected, false if done without detecting a loop
    */
    #[allow(dead_code)]
    fn traverse_blocker_tree(
        state: &RunState,
        visited_nodes: &mut Vec<usize>,
        root_node_id: usize,
        node: &mut BlockerNode,
    ) -> Result<Vec<BlockerNode>> {
        visited_nodes.push(node.function_id);
        node.blockers = Self::find_blockers(state, node.function_id)?;

        for blocker in &mut node.blockers {
            if blocker.function_id == root_node_id {
                return Ok(vec![blocker.clone()]); // add the last node in the loop to end of trail
            }

            // if we've visited this blocking node before, then we've detected a loop
            if !visited_nodes.contains(&blocker.function_id) {
                let mut blocker_subtree =
                    Self::traverse_blocker_tree(state, visited_nodes, root_node_id, blocker)?;
                if !blocker_subtree.is_empty() {
                    // insert this node at the head of the list of blocking nodes
                    blocker_subtree.insert(0, blocker.clone());
                    return Ok(blocker_subtree);
                }
            }
        }

        // no loop found
        Ok(vec![])
    }

    #[allow(dead_code)]
    fn display_set(root_node: &BlockerNode, node_set: Vec<BlockerNode>) -> String {
        let mut display_string = String::new();
        let _ = write!(display_string, "#{}", root_node.function_id);
        for node in node_set {
            let _ = write!(display_string, "{node}");
        }
        display_string
    }

    fn deadlock_check() -> String {
        " No deadlocks found\n".to_string()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::{json, Value};
    use url::Url;

    use flowcore::errors::Result;
    use flowcore::model::flow_manifest::FlowManifest;
    use flowcore::model::input::Input;
    use flowcore::model::input::InputInitializer::Once;
    use flowcore::model::metadata::MetaData;
    use flowcore::model::output_connection::OutputConnection;
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;

    use crate::block::Block;
    use crate::debug_command::{BreakpointSpec, DebugCommand};
    use crate::debugger::{BlockType, BlockerNode, Debugger};
    use crate::debugger_handler::DebuggerHandler;
    use crate::job::{Job, Payload};
    use crate::run_state::{RunState, State};

    struct DummyServer {
        job_breakpoint: usize,
        block_breakpoint: usize,
        send_breakpoint: (usize, usize), // (from id, to id)
        flow_unblock_breakpoint: usize,
        job_completed: bool,
        job_errored: bool,
        panicked: bool,
    }

    impl DummyServer {
        fn new() -> Self {
            DummyServer {
                job_breakpoint: usize::MAX,
                block_breakpoint: usize::MAX,
                send_breakpoint: (0, 0),
                flow_unblock_breakpoint: usize::MAX,
                job_completed: false,
                job_errored: false,
                panicked: false,
            }
        }
    }

    impl DebuggerHandler for DummyServer {
        fn start(&mut self) {}
        fn job_breakpoint(&mut self, job: &Job, _function: &RuntimeFunction, _states: Vec<State>) {
            self.job_breakpoint = job.payload.job_id;
        }
        fn block_breakpoint(&mut self, block: &Block) {
            self.block_breakpoint = block.blocked_function_id;
        }
        fn flow_unblock_breakpoint(&mut self, flow_id: usize) {
            self.flow_unblock_breakpoint = flow_id;
        }
        fn send_breakpoint(
            &mut self,
            _: &str,
            source_process_id: usize,
            _output_route: &str,
            _value: &Value,
            destination_id: usize,
            _destination_name: &str,
            _input_name: &str,
            _input_number: usize,
        ) {
            self.send_breakpoint = (source_process_id, destination_id);
        }
        fn job_error(&mut self, _job: &Job) {
            self.job_errored = true;
        }
        fn job_completed(&mut self, _job: &Job) {
            self.job_completed = true;
        }
        fn blocks(&mut self, _blocks: Vec<Block>) {}
        fn outputs(&mut self, _output: Vec<OutputConnection>) {}
        fn input(&mut self, _input: Input) {}
        fn function_list(&mut self, _functions: &[RuntimeFunction]) {}
        fn function_states(&mut self, _function: RuntimeFunction, _function_states: Vec<State>) {}
        fn run_state(&mut self, _run_state: &RunState) {}
        fn message(&mut self, _message: String) {}
        fn breakpoint_list(&mut self, _breakpoints: Vec<BreakpointSpec>) {}
        fn panic(&mut self, _state: &RunState, _error_message: String) {
            self.panicked = true;
        }
        fn debugger_exiting(&mut self) {}
        fn debugger_resetting(&mut self) {}
        fn debugger_error(&mut self, _error: String) {}
        fn execution_starting(&mut self) {}
        fn execution_ended(&mut self) {}
        fn process_tree(&mut self, _: &RunState) {}
        fn inspect_by_state(&mut self, _: &str, _: &RunState) {}
        fn inspect_flow(&mut self, _: usize, _: &RunState) {}
        fn get_command(&mut self, _state: &RunState) -> Result<DebugCommand> {
            Ok(DebugCommand::Step(None))
        }
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
        Submission::new(
            test_manifest(functions),
            None,
            None,
            #[cfg(feature = "debugger")]
            true,
        )
    }

    fn test_function(id: usize) -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fA",
            #[cfg(feature = "debugger")]
            "/fA",
            "file://fake/test",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "",
                0,
                false,
                Some(Once(json!(1))),
                None,
            )],
            id,
            0,
            &[],
            false,
        )
    }

    fn test_job() -> Job {
        Job {
            process_id: 0,
            #[cfg(feature = "debugger")]
            function_name: String::new(),
            parent_id: 0,
            connections: vec![],
            payload: Payload {
                job_id: 0,
                implementation_url: Url::parse("file://test").expect("Could not parse Url"),
                input_set: vec![json!(1)],
            },
            result: Ok((Some(json!(1)), true)),
        }
    }

    #[test]
    fn test_display_blocker_node() {
        let node = BlockerNode::new(0, BlockType::OutputBlocked);
        println!("{node}");
        let node = BlockerNode::new(0, BlockType::UnreadySender);
        println!("{node}");
    }

    #[test]
    fn test_check_prior_to_job() {
        let mut state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let job = test_job();
        let mut debugger = Debugger::new(&mut server);

        // configure the debugger to break at this job via it's ID
        debugger.break_at_job = job.payload.job_id;

        // call the debugger check
        let _ = debugger.check_prior_to_job(&mut state, &job);

        // check the breakpoint triggered at this job_id as expected
        assert_eq!(server.job_breakpoint, job.payload.job_id);
    }

    #[test]
    fn test_check_on_block_creation() {
        let mut state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        // configure a break on block creation from function #0 to function #1
        debugger.block_breakpoints.insert((0, 1));
        let block = Block::new(0, 1, 0, 0, 0);
        let _ = debugger.check_on_block_creation(&mut state, &block);

        // check the breakpoint triggered at this blocked function as expected
        assert_eq!(server.block_breakpoint, 0);
    }

    #[test]
    fn test_check_prior_to_send_output() {
        let mut state = RunState::new(test_submission(vec![test_function(0), test_function(1)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        // Set up a breakpoint on the output from function #0
        debugger.output_breakpoints.insert((0, String::new()));

        let _ = debugger.check_prior_to_send(&mut state, 0, "", &json!(1), 1, 0);

        // check the breakpoint triggered upon sending from function/route
        assert_eq!(server.send_breakpoint, (0, 1));
    }

    #[test]
    fn test_check_prior_to_send_input() {
        let mut state = RunState::new(test_submission(vec![test_function(0), test_function(1)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        // Set up a breakpoint on the input to function #0, input #0
        debugger.input_breakpoints.insert((0, 0));

        // send from an imaginary function #1 to function #0 input #0
        let _ = debugger.check_prior_to_send(&mut state, 1, "", &json!(1), 0, 0);

        // check the breakpoint triggered upon sending to the function/input
        assert_eq!(server.send_breakpoint, (1, 0));
    }

    #[test]
    fn test_check_prior_to_flow_unblock() {
        let mut state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        // Set up a breakpoint on the unblocking of flow #0
        debugger.flow_unblock_breakpoints.insert(0);

        let _ = debugger.check_prior_to_flow_unblock(&mut state, 0);

        // check the breakpoint triggered when the flow was unblocked as expected
        assert_eq!(server.flow_unblock_breakpoint, 0);
    }

    #[test]
    fn test_debugger_reset() {
        let mut server = DummyServer::new();
        let job = test_job();
        let mut debugger = Debugger::new(&mut server);

        // configure the debugger to break at this job via it's ID
        debugger.break_at_job = job.payload.job_id;
        debugger.block_breakpoints.insert((0, 1));
        debugger.output_breakpoints.insert((0, String::new()));
        debugger.input_breakpoints.insert((0, 0));
        debugger.flow_unblock_breakpoints.insert(0);

        debugger.reset();

        assert_eq!(debugger.break_at_job, usize::MAX);
        assert_eq!(debugger.block_breakpoints.len(), 1);
        assert_eq!(debugger.output_breakpoints.len(), 1);
        assert_eq!(debugger.input_breakpoints.len(), 1);
        assert_eq!(debugger.flow_unblock_breakpoints.len(), 1);
    }

    #[test]
    fn test_job_completed_ok() {
        let mut state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        let job = test_job();

        let _ = debugger.job_done(&mut state, &job);

        assert!(server.job_completed);
    }

    #[test]
    fn test_job_completed_err() {
        let mut state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        let mut job = test_job();
        job.result = Err(flowcore::errors::Error::from("Test fake Error"));

        let _ = debugger.job_done(&mut state, &job);

        assert!(server.job_errored);
    }

    #[test]
    fn test_panic() {
        let mut state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        let _ = debugger.error(&mut state, "Test error".into());

        assert!(server.panicked);
    }

    #[test]
    fn test_inspect_blocks_returns_empty() {
        let state = RunState::new(test_submission(vec![test_function(0)]));

        // Blocking has been removed, so inspect_blocks always returns empty
        assert!(Debugger::inspect_blocks(&state, Some(0), None).is_empty());
        assert!(Debugger::inspect_blocks(&state, None, None).is_empty());
    }

    #[test]
    fn test_none_breakpoint_spec_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, None).is_err());
    }

    #[test]
    fn test_all_breakpoint_spec_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::All))
            .is_err());
    }

    #[test]
    fn test_non_specific_block_breakpoint_spec_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Block((None, None))))
            .is_err());
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Block((None, Some(0)))))
            .is_err());
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), None))))
            .is_err());
    }

    #[test]
    fn test_specific_block_breakpoint_spec_passes() {
        let state = RunState::new(test_submission(vec![test_function(0), test_function(1)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1)))))
            .is_ok());
    }

    #[test]
    fn test_numeric_breakpoint_no_such_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Numeric(1)))
            .is_err());
    }

    #[test]
    fn test_numeric_breakpoint_existing_function_passes() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Numeric(0)))
            .is_ok());
    }

    #[test]
    fn test_input_breakpoint_no_such_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Input((1, 0))))
            .is_err());
    }

    #[test]
    fn test_input_breakpoint_no_such_input_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Input((0, 1))))
            .is_err());
    }

    #[test]
    fn test_input_breakpoint_passes() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Input((0, 0))))
            .is_ok());
    }

    #[test]
    fn test_block_breakpoint_no_such_source_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Block((Some(1), Some(0)))))
            .is_err());
    }

    #[test]
    fn test_block_breakpoint_no_such_destination_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1)))))
            .is_err());
    }

    #[test]
    fn test_output_breakpoint_no_such_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Output((1, String::new()))))
            .is_err());
    }

    #[test]
    fn test_output_breakpoint_function_exists_passes() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Output((0, String::new()))))
            .is_ok());
    }

    #[test]
    fn test_delete_none_breakpoint_spec_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, None).is_err());
    }

    #[test]
    fn test_delete_all_breakpoint_spec_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::All))
            .is_ok());
    }

    #[test]
    fn test_delete_non_specific_block_breakpoint_spec_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Block((None, None))))
            .is_err());
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Block((None, Some(0)))))
            .is_err());
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), None))))
            .is_err());
    }

    #[test]
    fn test_delete_specific_block_breakpoint_spec_passes() {
        let state = RunState::new(test_submission(vec![test_function(0), test_function(1)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1)))))
            .expect("Couldn't add breakpoint");
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1)))))
            .is_ok());
    }

    #[test]
    fn test_delete_numeric_breakpoint_no_such_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Numeric(1)))
            .is_err());
    }

    #[test]
    fn test_delete_numeric_breakpoint_existing_function_passes() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Numeric(0)))
            .expect("Couldn't add breakpoint");
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Numeric(0)))
            .is_ok());
    }

    #[test]
    fn test_delete_input_breakpoint_no_such_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Input((1, 0))))
            .is_err());
    }

    #[test]
    fn test_delete_input_breakpoint_no_such_input_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Input((0, 1))))
            .is_err());
    }

    #[test]
    fn test_delete_input_breakpoint_passes() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Input((0, 0))))
            .expect("Couldn't add breakpoint");
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Input((0, 0))))
            .is_ok());
    }

    #[test]
    fn test_delete_block_breakpoint_no_such_source_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Block((Some(1), Some(0)))))
            .is_err());
    }

    #[test]
    fn test_delete_block_breakpoint_no_such_destination_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1)))))
            .is_err());
    }

    #[test]
    fn test_delete_output_breakpoint_no_such_function_fails() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Output((1, String::new()))))
            .is_err());
    }

    #[test]
    fn test_delete_output_breakpoint_function_exists_passes() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Output((0, String::new()))))
            .expect("Couldn't add breakpoint");
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Output((0, String::new()))))
            .is_ok());
    }

    #[test]
    fn test_add_completed_breakpoint() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        let result = debugger.add_breakpoint(&state, Some(BreakpointSpec::Completed(0)));
        assert!(result.is_ok());
        assert!(debugger.completed_breakpoints.contains(&0));
    }

    #[test]
    fn test_add_completed_breakpoint_invalid_function() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        let result = debugger.add_breakpoint(&state, Some(BreakpointSpec::Completed(99)));
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_completed_breakpoint() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Completed(0)))
            .expect("Couldn't add breakpoint");
        assert!(debugger
            .delete_breakpoint(&state, Some(BreakpointSpec::Completed(0)))
            .is_ok());
        assert!(!debugger.completed_breakpoints.contains(&0));
    }

    #[test]
    fn test_list_completed_breakpoints() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger
            .add_breakpoint(&state, Some(BreakpointSpec::Completed(0)))
            .expect("Couldn't add breakpoint");
        let specs = debugger.collect_breakpoint_specs();
        assert!(specs.contains(&BreakpointSpec::Completed(0)));
    }

    #[test]
    fn test_process_tree() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let tree = Debugger::process_tree(&state);
        assert!(!tree.is_empty());
        assert!(tree.contains("#0"));
    }

    #[test]
    fn test_job_done_with_completed_breakpoint() {
        let mut state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let job = test_job();
        let mut debugger = Debugger::new(&mut server);
        debugger.completed_breakpoints.insert(0);
        let _ = debugger.job_done(&mut state, &job);
        assert!(server.job_completed);
        assert_eq!(server.job_breakpoint, job.payload.job_id);
    }

    #[test]
    fn test_inspect_by_state_ready() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let result = Debugger::inspect_by_state(&state, "ready");
        assert!(result.contains("Functions in 'ready' state:"));
    }

    #[test]
    fn test_inspect_by_state_waiting() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let result = Debugger::inspect_by_state(&state, "waiting");
        assert!(result.contains("Functions in 'waiting' state:"));
    }

    #[test]
    fn test_inspect_by_state_unknown() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let result = Debugger::inspect_by_state(&state, "bogus");
        assert!(result.contains("Unknown state"));
    }

    #[test]
    fn test_inspect_by_state_blocked() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let result = Debugger::inspect_by_state(&state, "blocked");
        assert!(result.contains("Blocked functions:"));
    }

    #[test]
    fn test_inspect_by_route_found() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let result = Debugger::inspect_by_route(&state, "/fA");
        assert!(result.contains("Function #0"));
    }

    #[test]
    fn test_inspect_by_route_not_found() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let result = Debugger::inspect_by_route(&state, "/nonexistent");
        assert!(result.contains("No function or flow found"));
    }

    #[test]
    fn test_find_by_route() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        assert_eq!(Debugger::find_by_route(&state, "/fA"), Some(0));
        assert_eq!(Debugger::find_by_route(&state, "/nonexistent"), None);
    }

    #[test]
    fn test_add_breakpoint_by_route() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        let result = debugger.add_breakpoint(&state, Some(BreakpointSpec::Route("/fA".into())));
        assert!(result.is_ok());
        assert!(debugger.function_breakpoints.contains(&0));
    }

    #[test]
    fn test_add_breakpoint_by_route_not_found() {
        let state = RunState::new(test_submission(vec![test_function(0)]));
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        let result =
            debugger.add_breakpoint(&state, Some(BreakpointSpec::Route("/nonexistent".into())));
        assert!(result.is_err());
    }
}
