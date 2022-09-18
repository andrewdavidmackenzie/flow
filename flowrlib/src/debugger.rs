use std::collections::HashSet;
use std::fmt;
use std::fmt::Write;

use error_chain::bail;
use log::error;
use serde_json::Value;

use flowcore::errors::*;
use flowcore::model::output_connection::Source::{Input, Output};

use crate::block::Block;
use crate::debug_command::BreakpointSpec;
use crate::debug_command::DebugCommand;
use crate::debug_command::DebugCommand::{Ack, Breakpoint, Continue, DebugClientStarting, Delete, Error, ExitDebugger, Inspect, InspectBlock, InspectFunction, InspectInput, InspectOutput, Invalid, List, Modify, RunReset, Step, Validate};
use crate::job::Job;
use crate::run_state::RunState;
use crate::server::DebuggerProtocol;

/// Debugger struct contains all the info necessary to conduct a debugging session, storing
/// set breakpoints, connections to the debug client etc
pub struct Debugger<'a> {
    debug_server: &'a mut dyn DebuggerProtocol,
    input_breakpoints: HashSet<(usize, usize)>,
    block_breakpoints: HashSet<(usize, usize)>,
    /* blocked_id -> blocking_id */
    output_breakpoints: HashSet<(usize, String)>,
    break_at_job: usize,
    function_breakpoints: HashSet<usize>,
    flow_unblock_breakpoints: HashSet<usize>,
}

#[derive(Debug, Clone)]
enum BlockType {
    OutputBlocked,
    // Cannot run and send it's Output as a destination Input is full
    UnreadySender, // Has to send output to an empty Input for other process to be able to run
}

#[derive(Debug, Clone)]
struct BlockerNode {
    function_id: usize,
    block_type: BlockType,
    blockers: Vec<BlockerNode>,
}

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
    pub fn new(
        debug_server: &'a mut dyn DebuggerProtocol,
    ) -> Self {
        Debugger {
            debug_server,
            input_breakpoints: HashSet::<(usize, usize)>::new(),
            block_breakpoints: HashSet::<(usize, usize)>::new(),
            output_breakpoints: HashSet::<(usize, String)>::new(),
            break_at_job: usize::MAX,
            function_breakpoints: HashSet::<usize>::new(),
            flow_unblock_breakpoints: HashSet::<usize>::new(),
        }
    }

    /// Start the debugger
    pub fn start(&mut self) {
        self.debug_server.start();
    }

    /// Check if there is a breakpoint at this job prior to starting executing it.
    /// Return values are (display next output, reset execution)
    pub fn check_prior_to_job(
        &mut self,
        state: &mut RunState,
        job: &Job,
    ) -> Result<(bool, bool)> {
        if self.break_at_job == job.job_id || self.function_breakpoints.contains(&job.function_id) {
            self.debug_server.job_breakpoint(job, state.get_function(job.function_id)
                .ok_or("Could not get function")?,
                                             state.get_function_states(job.function_id));
            return self.wait_for_command(state);
        }

        Ok((false, false))
    }

    /// Called from flowrlib during execution when it is about to create a block on one function
    /// due to not being able to send outputs to another function.
    ///
    /// This allows the debugger to check if we have a breakpoint set on that block. If we do
    /// then enter the debugger client and wait for a command.
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
            let source_function = state.get_function(source_function_id)
                .ok_or("Could not get function")?;
            let destination_function = state.get_function(destination_id)
                .ok_or("Could not get function")?;
            let io_name = destination_function.input(input_number).ok_or("Could not get input")?.name();

            self.debug_server.send_breakpoint(source_function.name(), source_function_id, output_route, value,
                                              destination_id, destination_function.name(),
                                              io_name, input_number);
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
            self.debug_server.flow_unblock_breakpoint(flow_being_unblocked_id);
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
    pub fn job_done(&mut self, state: &mut RunState, job: &Job) -> Result<(bool, bool)> {
        if job.result.is_err() {
            if state.submission.debug {
                let _ = self.job_error(state, job);
            }
        } else {
            self.debug_server.job_completed(job);
        }
        Ok((false, false))
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

    /// Execution of the flow ended, report it, check for deadlocks and wait for command
    /// Return values are (display next output, reset execution)
    pub fn execution_ended(&mut self, state: &mut RunState) -> Result<(bool, bool)> {
        self.debug_server.execution_ended();
        self.deadlock_check(state)?;
        self.wait_for_command(state)
    }

    /// The execution flow has entered the debugged based on some event.
    ///
    /// Now wait for and process commands from the DebugClient
    /// - execute and respond immediately those that require it
    /// - some commands will cause the command loop to exit.
    ///
    /// When exiting return a set of booleans for the Coordinator to determine what to do:
    /// (display next output, reset execution, exit_debugger)
    pub fn wait_for_command(&mut self, state: &mut RunState) -> Result<(bool, bool)> {
        loop {
            match self.debug_server.get_command(state)
            {
                // *************************      The following are commands that send a response
                Ok(Breakpoint(param)) => {
                    let result =  self.add_breakpoint(state, param);
                    let message = result.unwrap_or_else(|e| e.to_string());
                    self.debug_server.message(message);
                },
                Ok(Delete(param)) => {
                    let result = self.delete_breakpoint(state, param);
                    let message = result.unwrap_or_else(|e| e.to_string());
                    self.debug_server.message(message);
                },
                Ok(Validate) => {
                    let message = self.validate(state)?;
                    self.debug_server.message(message);
                },
                Ok(List) => {
                    let message = self.list_breakpoints();
                    self.debug_server.message(message);
                },
                Ok(DebugCommand::FunctionList) => {
                    self.debug_server.function_list(state.get_functions());
                },
                Ok(Inspect) => self.debug_server.run_state(state),
                Ok(InspectFunction(function_id)) => {
                    if function_id < state.num_functions() {
                        self.debug_server.function_states(state.get_function(function_id)
                                                              .ok_or("Could not get function")?.clone(),
                                                         state.get_function_states(function_id));
                    } else {
                        self.debug_server.debugger_error(format!("No function with id = {}", function_id));
                    };
                }
                Ok(InspectInput(function_id, input_number)) => {
                    if function_id < state.num_functions() {
                        let function = state.get_function(function_id)
                            .ok_or("Could not get function")?;

                        if input_number < function.inputs().len() {
                            self.debug_server.input(function.input(input_number)
                                .ok_or("Could not get input")?
                                .clone());
                        } else {
                            self.debug_server.debugger_error(format!(
                                "Function #{} has no input number {}", function_id, input_number
                            ));
                        }
                    } else {
                        self.debug_server.debugger_error(format!("No function with id = {}", function_id));
                    };
                }
                Ok(InspectOutput(function_id, sub_route)) => {
                    if function_id < state.num_functions() {
                        let function = state.get_function(function_id)
                            .ok_or("Could not get function")?;

                        let mut output_connections = vec![];

                        for output_connection in function.get_output_connections() {
                            match &output_connection.source {
                                Output(source_route) => {
                                    if *source_route == sub_route {
                                        output_connections.push(output_connection.clone())
                                    }
                                }
                                // add list of connections from an input to job if path "" is specified
                                Input(_) => {
                                    if sub_route.is_empty() {
                                        output_connections.push(output_connection.clone())
                                    }
                                }
                            }
                        }
                        self.debug_server.outputs(output_connections);
                    } else {
                        self.debug_server.debugger_error(format!("No function with id = {}", function_id));
                    };
                }
                Ok(InspectBlock(from_function_id, to_function_id)) => {
                    let blocks = Self::inspect_blocks(state, from_function_id, to_function_id);
                    self.debug_server.blocks(blocks);
                }
                Ok(Modify(specs)) => self.modify_variables(state, specs),
                Ok(Ack) => {}
                Ok(DebugClientStarting) => { // TODO remove
                    error!("Unexpected message 'DebugClientStarting' after started")
                }
                Ok(Error(_)) => { /* client error */ }
                Ok(Invalid) => {}
                Err(e) => error!("Error in Debug server getting command; {}", e),

                // ************************** The following commands may exit the command loop
                Ok(Continue) => {
                    if state.get_number_of_jobs_created() > 0 {
                        return Ok((false, false));
                    }
                }
                Ok(RunReset) => {
                    return if state.get_number_of_jobs_created() > 0 {
                        self.reset();
                        self.debug_server.debugger_resetting();
                        Ok((false, true))
                    } else {
                        self.debug_server.execution_starting();
                        Ok((false, false))
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
            };
        }
    }

    /*
       Find current blocks that match the spec. NOTE the source and destination function ids
       can both or either be None (for Any) or a specific function.

       If both are Any, then all blocks will match.
    */
    fn inspect_blocks(
        run_state: &RunState,
        from: Option<usize>,
        to: Option<usize>,
    ) -> Vec<Block> {
        let mut matching_blocks = vec![];

        for block in run_state.get_blocks() {
            if (from.is_none() || from == Some(block.blocked_function_id))
                && (to.is_none() || to == Some(block.blocking_function_id))
            {
                matching_blocks.push(block.clone());
            }
        }

        matching_blocks
    }

    /****************************** Implementations of Debugger Commands *************************/

    /*
       Add a breakpoint to the debugger according to the Optional `Param`
    */
    fn add_breakpoint(&mut self, state: &RunState, param: Option<BreakpointSpec>) -> Result<String> {
        match param {
            None => bail!("'break' command must specify a breakpoint\n"),
            Some(BreakpointSpec::All) =>
                bail!("To break on every Function, you can just single step using 's' command\n"),
            Some(BreakpointSpec::Numeric(process_id)) => {
                if process_id >= state.num_functions() {
                    bail!("There is no Function with id '{process_id}' to set a breakpoint on");
                }

                self.function_breakpoints.insert(process_id);
                let function = state.get_function(process_id)
                    .ok_or("Could not get function")?;
                Ok(format!("Breakpoint set on Function #{} ({}) @ '{}'",
                    process_id, function.name(), function.route()))
            }
            Some(BreakpointSpec::Input((destination_id, input_number))) => {
                if destination_id >= state.num_functions() {
                    bail!("There is no Function #{destination_id} to set a breakpoint on");
                }

                let function = state.get_function(destination_id)
                    .ok_or("Could not get function")?;

                if input_number >= function.inputs().len() {
                    bail!("There is no Input :{input_number} on function #{destination_id}");
                }
                let io_name = function.input(input_number).ok_or("Could not get input")?.name();
                self.input_breakpoints.insert((destination_id, input_number));
                Ok(format!(
                    "Data breakpoint set on Function #{destination_id}:{input_number} '{}' receiving data on input '{}'",
                    function.name(), io_name))
            }
            Some(BreakpointSpec::Block((Some(blocked_id), Some(blocking_id)))) => {
                if blocked_id >= state.num_functions() {
                    bail!("There is no Function #{blocked_id} to set a Block breakpoint on");
                }

                if blocking_id >= state.num_functions() {
                    bail!("There is no Function #{blocking_id} to set a Block breakpoint on");
                }

                self.block_breakpoints.insert((blocked_id, blocking_id));
                Ok(format!(
                    "Block breakpoint set on Function #{blocked_id} being blocked by Function #{blocking_id}"))
            }
            Some(BreakpointSpec::Block(_)) => bail!("Invalid format to set a breakpoint on a block\n"),
            Some(BreakpointSpec::Output((source_id, source_output_route))) => {
                if source_id >= state.num_functions() {
                    bail!("There is no Function #{source_id} to set a Output breakpoint on");
                }

                self.output_breakpoints.insert((source_id, source_output_route.clone()));
                Ok(format!(
                    "Data breakpoint set on Function #{source_id} sending data via output: '{source_output_route}'"
                ))
            }
        }
    }

    /*
       Delete debugger breakpoints related to Jobs or Blocks, etc according to the Spec.
    */
    fn delete_breakpoint(&mut self, state: &RunState, param: Option<BreakpointSpec>) -> Result<String> {
        match param {
            None => bail!("No process id specified\n"),
            Some(BreakpointSpec::All) => {
                self.output_breakpoints.clear();
                self.input_breakpoints.clear();
                self.function_breakpoints.clear();
                Ok("Deleted all breakpoints\n".into())
            }
            Some(BreakpointSpec::Numeric(process_number)) => {
                if process_number >= state.num_functions() {
                    bail!("There is no Function with id '{process_number}' to delete a breakpoint from");
                }

                if self.function_breakpoints.remove(&process_number) {
                    Ok(format!("Breakpoint on process #{process_number} was deleted"))
                } else {
                    bail!("No breakpoint number '{}' exists\n")
                }
            }
            Some(BreakpointSpec::Input((destination_id, input_number))) => {
                if destination_id >= state.num_functions() {
                    bail!("There is no Function #{destination_id} to delete a breakpoint from");
                }

                let function = state.get_function(destination_id)
                    .ok_or("Could not get function")?;

                if input_number >= function.inputs().len() {
                    bail!("There is no Input :{input_number} on function #{destination_id}");
                }

                self.input_breakpoints
                    .remove(&(destination_id, input_number));
                Ok("Inputs breakpoint removed\n".into())
            }
            Some(BreakpointSpec::Block((Some(blocked_id), Some(blocking_id)))) => {
                if blocked_id >= state.num_functions() {
                    bail!("There is no Function #{blocked_id} to delete a Block breakpoint from");
                }

                if blocking_id >= state.num_functions() {
                    bail!("There is no Function #{blocking_id} to delete a Block breakpoint from");
                }

                self.input_breakpoints.remove(&(blocked_id, blocking_id));
                Ok("Inputs breakpoint removed\n".into())
            }
            Some(BreakpointSpec::Block(_)) => bail!("Invalid format to remove breakpoint\n"),
            Some(BreakpointSpec::Output((source_id, source_output_route))) => {
                if source_id >= state.num_functions() {
                    bail!("There is no Function #{source_id} to delete a Output breakpoint from");
                }

                self.output_breakpoints
                    .remove(&(source_id, source_output_route));
                Ok("Output breakpoint removed\n".into())
            }
        }
    }

    /*
       List all debugger breakpoints of all types.
       // TODO make structs not a string
    */
    fn list_breakpoints(&self) -> String {
        let mut response = String::new();

        let mut breakpoints = false;
        if !self.function_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Function Breakpoints: \n");
            for process_id in &self.function_breakpoints {
                let _ = writeln!(response, "\tFunction #{}", process_id);
            }
        }

        if !self.output_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Output Breakpoints: \n");
            for (process_id, route) in &self.output_breakpoints {
                let _ = writeln!(response, "\tOutput #{}/{}", process_id, route);
            }
        }

        if !self.input_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Input Breakpoints: \n");
            for (process_id, input_number) in &self.input_breakpoints {
                let _ = writeln!(response, "\tInput #{}:{}", process_id, input_number);
            }
        }

        if !self.block_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Block Breakpoints: \n");
            for (blocked_id, blocking_id) in &self.block_breakpoints {
                let _ = writeln!(response, "\tBlock #{}->#{}", blocked_id, blocking_id);
            }
        }

        if !breakpoints {
            response.push_str(
                "No Breakpoints set. Use the 'b' command to set a breakpoint. Use 'h' for help.\n",
            );
        }

        response
    }

    /*
       Run checks on the current flow execution state to check if it is valid
       Currently deadlock check is the only check that exists.
    */
    fn validate(&self, state: &RunState) -> Result<String> {
        let mut response = String::new();

        response.push_str("Validating flow state\n");
        response.push_str("Running deadlock check...  ");
        response.push_str(&self.deadlock_check(state)?);

        Ok(response)
    }

    // Get ready to start execution (and debugging) from scratch at the start of the flow
    fn reset(&mut self) {
        // Leave all the breakpoints untouched for the repeat run
        self.break_at_job = usize::MAX;
    }

    // Parse a series of specs to modify a state value
    fn modify_variables(&mut self, state: &mut RunState, specs: Option<Vec<String>>) {
        match specs.as_deref() {
            None | Some([]) => self.debug_server.message("State variables that can be modified are:\
            \n'jobs' - maximum number of parallel jobs (integer) or 0 for no limit".to_string()),
            Some(specs) => {
                for spec in specs {
                    let parts: Vec<&str> = spec.trim().split('=').collect();
                    if parts.len() < 2 {
                        self.debug_server.message(
                            format!("Invalid modify command for state variables: '{}'", spec));
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
                                    self.debug_server.message(format!("State variable 'jobs' set to {}",
                                                                      var));
                                } else {
                                    self.debug_server.message(
                                        format!("Invalid value '{}' for variable 'jobs'", var));
                                }
                            }
                        }
                        _ => self.debug_server.message("Unknown state variable".to_string())
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
                    self.debug_server.debugger_error(
                        "Number of jobs to 'step' must be greater than 0\n".into());
                }
            }
        }
    }

    /*
        Return a vector of all the processes preventing process_id from running, which can be:
        - other process has input full and hence is blocking running of this process
        - other process is the only process that sends to an empty input of this process
    */
    fn find_blockers(&self, state: &RunState, process_id: usize) -> Result<Vec<BlockerNode>> {
        let mut blockers: Vec<BlockerNode> = state
            .get_output_blockers(process_id)
            .iter()
            .map(|(id, _)| BlockerNode::new(*id, BlockType::OutputBlocked))
            .collect();

        let input_blockers: Vec<BlockerNode> = state
            .get_input_blockers(process_id)?
            .iter()
            .map(|(id, _)| BlockerNode::new(*id, BlockType::UnreadySender))
            .collect();

        blockers.extend(input_blockers);

        Ok(blockers)
    }

    /*
        Traverse the tree of processes blocking this process from running, either because:
        - this process wants to send to the other, but the input it full
        - this process needs an input from the other

        Return true if a loop was detected, false if done without detecting a loop
    */
    fn traverse_blocker_tree(
        &self,
        state: &RunState,
        visited_nodes: &mut Vec<usize>,
        root_node_id: usize,
        node: &mut BlockerNode,
    ) -> Result<Vec<BlockerNode>> {
        visited_nodes.push(node.function_id);
        node.blockers = self.find_blockers(state, node.function_id)?;

        for blocker in &mut node.blockers {
            if blocker.function_id == root_node_id {
                return Ok(vec![blocker.clone()]); // add the last node in the loop to end of trail
            }

            // if we've visited this blocking node before, then we've detected a loop
            if !visited_nodes.contains(&blocker.function_id) {
                let mut blocker_subtree =
                    self.traverse_blocker_tree(state, visited_nodes, root_node_id, blocker)?;
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

    fn display_set(root_node: &BlockerNode, node_set: Vec<BlockerNode>) -> String {
        let mut display_string = String::new();
        let _ = write!(display_string, "#{}", root_node.function_id);
        for node in node_set {
            let _ = write!(display_string, "{}", node);
        }
        display_string
    }

    fn deadlock_check(&self, state: &RunState) -> Result<String> {
        let mut response = String::new();

        for blocked_process_id in state.get_blocked() {
            // start a clean tree with a new root node for each blocked process
            let mut root_node = BlockerNode::new(*blocked_process_id, BlockType::OutputBlocked);
            let mut visited_nodes = vec![];

            let deadlock_set = self.traverse_blocker_tree(
                state,
                &mut visited_nodes,
                *blocked_process_id,
                &mut root_node,
            )?;
            if !deadlock_set.is_empty() {
                let _ = writeln!(response, "{}", Self::display_set(&root_node, deadlock_set));
            }
        }

        if response.is_empty() {
           let _ = writeln!(response, " No deadlocks found");
        }

        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use serde_json::{json, Value};
    use url::Url;

    use flowcore::{Implementation, RunAgain};
    use flowcore::errors::Result;
    use flowcore::model::input::Input;
    use flowcore::model::input::InputInitializer::Once;
    use flowcore::model::output_connection::OutputConnection;
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;

    use crate::block::Block;
    use crate::debug_command::{BreakpointSpec, DebugCommand};
    use crate::debugger::{BlockerNode, BlockType, Debugger};
    use crate::job::Job;
    use crate::run_state::{RunState, State};
    use crate::server::DebuggerProtocol;

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
            DummyServer{
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

    impl DebuggerProtocol for DummyServer {
        fn start(&mut self) {}
        fn job_breakpoint(&mut self, job: &Job, _function: &RuntimeFunction, _states: Vec<State>) {
            self.job_breakpoint = job.job_id;
        }
        fn block_breakpoint(&mut self, block: &Block) {
            self.block_breakpoint = block.blocked_function_id;
        }
        fn flow_unblock_breakpoint(&mut self, flow_id: usize) {
            self.flow_unblock_breakpoint = flow_id;
        }
        fn send_breakpoint(&mut self, _: &str, source_process_id: usize, _output_route: &str, _value: &Value,
                           destination_id: usize, _destination_name:&str, _input_name: &str, _input_number: usize) {
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
        fn panic(&mut self, _state: &RunState, _error_message: String) {
            self.panicked = true;
        }
        fn debugger_exiting(&mut self) {}
        fn debugger_resetting(&mut self) {}
        fn debugger_error(&mut self, _error: String) {}
        fn execution_starting(&mut self) {}
        fn execution_ended(&mut self) {}
        fn get_command(&mut self, _state: &RunState) -> Result<DebugCommand> {
            Ok(DebugCommand::Step(None))
        }
    }

    fn test_function(id: usize) -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
                "fA",
            #[cfg(feature = "debugger")]
                "/fA",
            "file://fake/test",
            vec![Input::new(
                #[cfg(feature = "debugger")] "", 0, false,
                Some(Once(json!(1))), None)],
            id,
            0,
            &[],
            false,
        )
    }

    fn test_submission() -> Submission {
        Submission::new(
            &Url::parse("file:///temp/fake.toml").expect("Could not create Url"),
            None,
            #[cfg(feature = "debugger")]
                true,
        )
    }

    #[derive(Debug)]
    struct TestImpl {}

    impl Implementation for TestImpl {
        fn run(&self, _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
            unimplemented!()
        }
    }

    fn test_impl() -> Arc<dyn Implementation> {
        Arc::new(TestImpl {})
    }

    fn test_job() -> Job {
        Job {
            job_id: 0,
            function_id: 0,
            flow_id: 0,
            implementation: test_impl(),
            input_set: vec![json!(1)],
            result: Ok((Some(json!(1)), true)),
            connections: vec![],
        }
    }

    #[test]
    fn test_display_blocker_node() {
        let node = BlockerNode::new(0, BlockType::OutputBlocked);
        println!("{}", node);
        let node = BlockerNode::new(0, BlockType::UnreadySender);
        println!("{}", node);
    }

    #[test]
    fn test_check_prior_to_job() {
        let functions = vec![test_function(0)];
        let mut state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let job = test_job();
        let mut debugger = Debugger::new(&mut server);

        // configure the debugger to break at this job via it's ID
        debugger.break_at_job = job.job_id;

        // call the debugger check
        let _ = debugger.check_prior_to_job(&mut state, &job);

        // check the breakpoint triggered at this job_id as expected
        assert_eq!(server.job_breakpoint, job.job_id)
    }

    #[test]
    fn test_check_on_block_creation() {
        let functions = vec![test_function(0)];
        let mut state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        // configer a break on block creation from function #0 to function #1
        debugger.block_breakpoints.insert((0, 1));
        let block = Block::new(0, 1, 0, 0, 0);
        let _ = debugger.check_on_block_creation(&mut state, &block);

        // check the breakpoint triggered at this blocked function as expected
        assert_eq!(server.block_breakpoint, 0)
    }

    #[test]
    fn test_check_prior_to_send_output() {
        let functions = vec![test_function(0), test_function(1)];
        let mut state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        // Setup a breakpoint on the output from function #0
        debugger.output_breakpoints.insert((0, "".into()));

        let _ = debugger.check_prior_to_send(&mut state, 0, "",
                    &json!(1), 1, 0);

        // check the breakpoint triggered upon sending from function/route
        assert_eq!(server.send_breakpoint, (0, 1));
    }

    #[test]
    fn test_check_prior_to_send_input() {
        let functions = vec![test_function(0), test_function(1)];
        let mut state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        // Setup a breakpoint on the input to function #0, input #0
        debugger.input_breakpoints.insert((0, 0));

        // send from an imaginary function #1 to function #0 input #0
        let _ = debugger.check_prior_to_send(&mut state, 1, "",
                                             &json!(1), 0, 0);

        // check the breakpoint triggered upon sending to the function/input
        assert_eq!(server.send_breakpoint, (1, 0))
    }

    #[test]
    fn test_check_prior_to_flow_unblock() {
        let functions = vec![test_function(0)];
        let mut state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        // Setup a breakpoint on the unblocking of flow #0
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
        debugger.break_at_job = job.job_id;
        debugger.block_breakpoints.insert((0, 1));
        debugger.output_breakpoints.insert((0, "".into()));
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
        let functions = vec![test_function(0)];
        let mut state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        let job = test_job();

        let _ = debugger.job_done(&mut state, &job);

        assert!(server.job_completed);
    }

    #[test]
    fn test_job_completed_err() {
        let functions = vec![test_function(0)];
        let mut state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        let mut job = test_job();
        job.result = Err(flowcore::errors::Error::from("Test fake Error"));

        let _ = debugger.job_done(&mut state, &job);

        assert!(server.job_errored);
    }

    #[test]
    fn test_panic() {
        let functions = vec![test_function(0)];
        let mut state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        let _ = debugger.error(&mut state, "Test error".into());

        assert!(server.panicked);
    }

    #[test]
    fn test_inspect_blocks() {
        let functions = vec![test_function(0)];
        let mut state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);

        // zero_to_one
        let _ = state.create_block(0, 1, 0, 0, 0, &mut debugger);

        // zero_to_two
        let _ = state.create_block(0, 2, 0, 0, 0, &mut debugger);


        // three_to_two
        let _ = state.create_block(0, 2, 0, 3, 0, &mut debugger);

        // three_to_one
        let _ = state.create_block(0, 1, 0, 3, 0, &mut debugger);

        assert_eq!(Debugger::inspect_blocks(&state, Some(0), None).len(), 2);
        assert_eq!(Debugger::inspect_blocks(&state, Some(0), Some(1)).len(), 1);
        assert_eq!(Debugger::inspect_blocks(&state, Some(2), None).len(), 0);
        assert_eq!(Debugger::inspect_blocks(&state, None, Some(2)).len(), 2);
        assert_eq!(Debugger::inspect_blocks(&state, Some(3), Some(2)).len(), 1);
        assert_eq!(Debugger::inspect_blocks(&state, None, Some(0)).len(), 0);
    }

    #[test]
    fn test_none_breakpoint_spec_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, None).is_err());
    }

    #[test]
    fn test_all_breakpoint_spec_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::All)).is_err());
    }

    #[test]
    fn test_non_specific_block_breakpoint_spec_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Block((None, None)))).is_err());
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Block((None, Some(0))))).is_err());
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), None)))).is_err());
    }

    #[test]
    fn test_specific_block_breakpoint_spec_passes() {
        let functions = vec![test_function(0), test_function(1)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1))))).is_ok());
    }

    #[test]
    fn test_numeric_breakpoint_no_such_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Numeric(1))).is_err());
    }

    #[test]
    fn test_numeric_breakpoint_existing_function_passes() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Numeric(0))).is_ok());
    }

    #[test]
    fn test_input_breakpoint_no_such_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Input((1, 0)))).is_err());
    }

    #[test]
    fn test_input_breakpoint_no_such_input_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Input((0, 1)))).is_err());
    }

    #[test]
    fn test_input_breakpoint_passes() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Input((0, 0)))).is_ok());
    }

    #[test]
    fn test_block_breakpoint_no_such_source_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Block((Some(1), Some(0))))).is_err());
    }

    #[test]
    fn test_block_breakpoint_no_such_destination_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1))))).is_err());
    }

    #[test]
    fn test_output_breakpoint_no_such_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Output((1, "".into())))).is_err());
    }

    #[test]
    fn test_output_breakpoint_function_exists_passes() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.add_breakpoint(&state, Some(BreakpointSpec::Output((0, "".into())))).is_ok());
    }

    #[test]
    fn test_delete_none_breakpoint_spec_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, None).is_err());
    }

    #[test]
    fn test_delete_all_breakpoint_spec_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::All)).is_ok());
    }

    #[test]
    fn test_delete_non_specific_block_breakpoint_spec_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Block((None, None)))).is_err());
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Block((None, Some(0))))).is_err());
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), None)))).is_err());
    }

    #[test]
    fn test_delete_specific_block_breakpoint_spec_passes() {
        let functions = vec![test_function(0), test_function(1)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger.add_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1)))))
            .expect("Couldn't add breakpoint");
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1))))).is_ok());
    }

    #[test]
    fn test_delete_numeric_breakpoint_no_such_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Numeric(1))).is_err());
    }

    #[test]
    fn test_delete_numeric_breakpoint_existing_function_passes() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger.add_breakpoint(&state, Some(BreakpointSpec::Numeric(0)))
            .expect("Couldn't add breakpoint");
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Numeric(0))).is_ok());
    }

    #[test]
    fn test_delete_input_breakpoint_no_such_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Input((1, 0)))).is_err());
    }

    #[test]
    fn test_delete_input_breakpoint_no_such_input_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Input((0, 1)))).is_err());
    }

    #[test]
    fn test_delete_input_breakpoint_passes() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger.add_breakpoint(&state, Some(BreakpointSpec::Input((0, 0))))
            .expect("Couldn't add breakpoint");
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Input((0, 0)))).is_ok());
    }

    #[test]
    fn test_delete_block_breakpoint_no_such_source_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Block((Some(1), Some(0))))).is_err());
    }

    #[test]
    fn test_delete_block_breakpoint_no_such_destination_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Block((Some(0), Some(1))))).is_err());
    }

    #[test]
    fn test_delete_output_breakpoint_no_such_function_fails() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Output((1, "".into())))).is_err());
    }

    #[test]
    fn test_delete_output_breakpoint_function_exists_passes() {
        let functions = vec![test_function(0)];
        let state = RunState::new(&functions, test_submission());
        let mut server = DummyServer::new();
        let mut debugger = Debugger::new(&mut server);
        debugger.add_breakpoint(&state, Some(BreakpointSpec::Output((0, "".into()))))
            .expect("Couldn't add breakpoint");
        assert!(debugger.delete_breakpoint(&state, Some(BreakpointSpec::Output((0, "".into())))).is_ok());
    }
}