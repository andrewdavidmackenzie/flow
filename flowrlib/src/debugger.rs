use std::collections::HashSet;
use std::fmt;

use log::error;
use serde_json::Value;

use flowcore::model::output_connection::Source::{Input, Output};

use crate::block::Block;
use crate::debug_command::DebugCommand;
use crate::debug_command::DebugCommand::{Ack, Breakpoint, Continue, DebugClientStarting, Delete,
                                         Error, ExitDebugger, Inspect, InspectBlock, InspectFunction, InspectInput,
                                         InspectOutput, Invalid, List, RunReset, Step, Validate
                                    };
use crate::job::Job;
use crate::param::Param;
use crate::run_state::RunState;
use crate::server::DebugServer;

/// Debugger struct contains all the info necessary to conduct a debugging session, storing
/// set breakpoints, connections to the debug client etc
pub struct Debugger<'a> {
    debug_server: &'a mut dyn DebugServer,
    input_breakpoints: HashSet<(usize, usize)>,
    block_breakpoints: HashSet<(usize, usize)>,
    /* blocked_id -> blocking_id */
    output_breakpoints: HashSet<(usize, String)>,
    break_at_job: usize,
    function_breakpoints: HashSet<usize>,
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
        debug_server: &'a mut dyn DebugServer,
    ) -> Self {
        Debugger {
            debug_server,
            input_breakpoints: HashSet::<(usize, usize)>::new(),
            block_breakpoints: HashSet::<(usize, usize)>::new(),
            output_breakpoints: HashSet::<(usize, String)>::new(),
            break_at_job: usize::MAX,
            function_breakpoints: HashSet::<usize>::new(),
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
        state: &RunState,
        job: &Job,
    ) -> (bool, bool, bool) {
        if self.break_at_job == job.job_id || self.function_breakpoints.contains(&job.function_id) {
            self.debug_server.job_breakpoint(job, state.get_function(job.function_id),
                                             state.get_function_states(job.function_id));
            return self.wait_for_command(state);
        }

        (false, false, false)
    }

    /// Called from flowrlib during execution when it is about to create a block on one function
    /// due to not being able to send outputs to another function.
    ///
    /// This allows the debugger to check if we have a breakpoint set on that block. If we do
    /// then enter the debugger client and wait for a command.
    pub fn check_on_block_creation(
        &mut self,
        state: &RunState,
        block: &Block,
    ) -> (bool, bool, bool) {
        if self
            .block_breakpoints
            .contains(&(block.blocked_id, block.blocking_id))
        {
            self.debug_server.block_breakpoint(block);
            return self.wait_for_command(state);
        }

        (false, false, false)
    }

    /// Called from flowrlib runtime prior to sending a value to another function,to see if there
    /// is a breakpoint on that send.
    ///
    /// If there is, then enter the debug client and wait for a command.
    pub fn check_prior_to_send(
        &mut self,
        state: &RunState,
        source_function_id: usize,
        output_route: &str,
        value: &Value,
        destination_id: usize,
        input_number: usize,
    ) -> (bool, bool, bool) {
        if self
            .output_breakpoints
            .contains(&(source_function_id, output_route.to_string()))
            || self
                .input_breakpoints
                .contains(&(destination_id, input_number))
        {
            let source_function = state.get_function(source_function_id);
            let destination_function = state.get_function(destination_id);
            let io_name = destination_function.input(input_number).name();

            self.debug_server.send_breakpoint(source_function.name(), source_function_id, output_route, value,
                                              destination_id, destination_function.name(),
                                              io_name, input_number);
            return self.wait_for_command(state);
        }

        (false, false, false)
    }

    /// An error occurred while executing a flow. Let the debug client know, enter the client
    /// and wait for a user command.
    ///
    /// This is useful for debugging a flow that has an error. Without setting any explicit
    /// breakpoint it will enter the debugger on an error and let the user inspect the flow's
    /// state etc.
    pub fn job_error(&mut self, state: &RunState, job: &Job) -> (bool, bool, bool) {
        self.debug_server.job_error(job);
        self.wait_for_command(state)
    }

    /// Called from the flowrlib coordinator to inform the debug client that a job has completed
    /// Return values are (display next output, reset execution)
    pub fn job_completed(&mut self, state: &RunState, job: &Job) -> (bool, bool, bool) {
        if job.result.is_err() {
           let _ = self.job_error(state, job);
        } else {
            self.debug_server.job_completed(job);
        }
        (false, false, false)
    }

    /// A panic occurred while executing a flow. Let the debug client know, enter the client
    /// and wait for a user command.
    ///
    /// This is useful for debugging a flow that has an error. Without setting any explicit
    /// breakpoint it will enter the debugger on an error and let the user inspect the flow's
    /// state etc.
    pub fn panic(&mut self, state: &RunState, error_message: String) -> (bool, bool, bool) {
        self.debug_server.panic(state, error_message);
        self.wait_for_command(state)
    }

    /// Execution of the flow ended, report it, check for deadlocks and wait for command
    /// Return values are (display next output, reset execution)
    pub fn execution_ended(&mut self, state: &RunState) -> (bool, bool, bool) {
        self.debug_server.execution_ended();
        self.deadlock_check(state);
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
    pub fn wait_for_command(&mut self, state: &RunState) -> (bool, bool, bool) {
        loop {
            match self.debug_server.get_command(state)
            {
                // *************************      The following are commands that send a response
                Ok(Breakpoint(param)) => {
                    let message = self.add_breakpoint(state, param);
                    self.debug_server.message(message);
                },
                Ok(Delete(param)) => {
                    let message = self.delete_breakpoint(param);
                    self.debug_server.message(message);
                },
                Ok(Validate) => {
                    let message = self.validate(state);
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
                        self.debug_server.function_states(state.get_function(function_id).clone(),
                                                         state.get_function_states(function_id));
                    } else {
                        self.debug_server.debugger_error(format!("No function with id = {}", function_id));
                    };
                }
                Ok(InspectInput(function_id, input_number)) => {
                    if function_id < state.num_functions() {
                        let function = state.get_function(function_id);

                        if input_number < function.inputs().len() {
                            self.debug_server.input(function.input(input_number).clone());
                        } else {
                            self.debug_server.debugger_error(format!(
                                "Function #{function_id} has no input number {input_number}"
                            ));
                        }
                    } else {
                        self.debug_server.debugger_error(format!("No function with id = {function_id}"));
                    };
                }
                Ok(InspectOutput(function_id, sub_route)) => {
                    if function_id < state.num_functions() {
                        let function = state.get_function(function_id);

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
                        self.debug_server.debugger_error(format!("No function with id = {function_id}"));
                    };
                }
                Ok(InspectBlock(from_function_id, to_function_id)) => {
                    let blocks = Self::inspect_blocks(state, from_function_id, to_function_id);
                    self.debug_server.blocks(blocks);
                }
                Ok(ExitDebugger) => {
                    self.debug_server.debugger_exiting();
                    return (false, false, true);
                }

                // **************************      The following commands exit the command loop
                Ok(Continue) => {
                    if state.jobs_created() > 0 {
                        return (false, false, false);
                    }
                }
                Ok(RunReset) => {
                    return if state.jobs_created() > 0 {
                        self.reset();
                        self.debug_server.debugger_resetting();
                        (false, true, false)
                    } else {
                        self.debug_server.execution_starting();
                        (false, false, false)
                    }
                }
                Ok(Step(param)) => {
                    self.step(state, param);
                    return (true, false, false);
                }
                Ok(Ack) => {}
                Ok(DebugClientStarting) => { // TODO remove
                    error!("Unexpected message 'DebugClientStarting' after started")
                }
                Ok(Error(_)) => { /* client error */ }
                Ok(Invalid) => {}
                Err(e) => error!("Error in Debug server getting command; {e}"),
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
            if (from.is_none() || from == Some(block.blocked_id))
                && (to.is_none() || to == Some(block.blocking_id))
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
    fn add_breakpoint(&mut self, state: &RunState, param: Option<Param>) -> String {
        let mut response = String::new();

        match param {
            None => response.push_str("'break' command must specify a breakpoint\n"),
            Some(Param::Numeric(function_id)) => {
                if function_id > state.num_functions() {
                    response.push_str(&format!(
                        "There is no Function with id '{function_id}' to set a breakpoint on\n"
                    ));
                } else {
                    self.function_breakpoints.insert(function_id);
                    let function = state.get_function(function_id);
                    response.push_str(&format!(
                        "Breakpoint set on Function #{function_id} ({}) @ '{}'\n",
                        function.name(), function.route()
                    ));
                }
            }
            Some(Param::Input((destination_id, input_number))) => {
                let function = state.get_function(destination_id);
                let io_name = function.input(input_number).name();
                response.push_str(&format!(
                    "Data breakpoint set on Function #{destination_id}:{input_number} '{}' receiving data on input '{io_name}'\n",
                    function.name()));
                self.input_breakpoints
                    .insert((destination_id, input_number));
            }
            Some(Param::Block((Some(blocked_id), Some(blocking_id)))) => {
                response.push_str(&format!(
                    "Block breakpoint set on Function #{blocked_id} being blocked by Function #{blocking_id}\n",
                ));
                self.block_breakpoints.insert((blocked_id, blocking_id));
            }
            Some(Param::Block(_)) => {
                response.push_str("Invalid format to set a breakpoint on a block\n");
            }
            Some(Param::Output((source_id, source_output_route))) => {
                response.push_str(&format!(
                    "Data breakpoint set on Function #{source_id} sending data via output: '{source_output_route}'\n",
                ));
                self.output_breakpoints
                    .insert((source_id, source_output_route));
            }
            Some(Param::Wildcard) => {
                response.push_str(
                    "To break on every Function, you can just single step using 's' command\n",
                );
            }
        }

        response
    }

    /*
       Delete debugger breakpoints related to Jobs or Blocks, etc according to the Spec.
    */
    fn delete_breakpoint(&mut self, param: Option<Param>) -> String {
        let mut response = String::new();

        match param {
            None => response.push_str("No process id specified\n"),
            Some(Param::Numeric(process_number)) => {
                if self.function_breakpoints.remove(&process_number) {
                    response.push_str(&format!(
                        "Breakpoint on function #{process_number} was deleted\n",
                    ));
                } else {
                    response.push_str("No breakpoint number '{}' exists\n");
                }
            }
            Some(Param::Input((destination_id, input_number))) => {
                self.input_breakpoints
                    .remove(&(destination_id, input_number));
                response.push_str("Inputs breakpoint removed\n");
            }
            Some(Param::Block((Some(blocked_id), Some(blocking_id)))) => {
                self.input_breakpoints.remove(&(blocked_id, blocking_id));
                response.push_str("Inputs breakpoint removed\n");
            }
            Some(Param::Block(_)) => {
                response.push_str("Invalid format to remove breakpoint\n");
            }
            Some(Param::Output((source_id, source_output_route))) => {
                self.output_breakpoints
                    .remove(&(source_id, source_output_route));
                response.push_str("Output breakpoint removed\n");
            }
            Some(Param::Wildcard) => {
                self.output_breakpoints.clear();
                self.input_breakpoints.clear();
                self.function_breakpoints.clear();
                response.push_str("Deleted all breakpoints\n");
            }
        }

        response
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
                response.push_str(&format!("\tFunction #{process_id}\n"));
            }
        }

        if !self.output_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Output Breakpoints: \n");
            for (process_id, route) in &self.output_breakpoints {
                response.push_str(&format!("\tOutput #{process_id}/{route}\n"));
            }
        }

        if !self.input_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Input Breakpoints: \n");
            for (process_id, input_number) in &self.input_breakpoints {
                response.push_str(&format!("\tInput #{process_id}:{input_number}\n"));
            }
        }

        if !self.block_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Block Breakpoints: \n");
            for (blocked_id, blocking_id) in &self.block_breakpoints {
                response.push_str(&format!("\tBlock #{blocked_id}->#{blocking_id}\n"));
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
    fn validate(&self, state: &RunState) -> String {
        let mut response = String::new();

        response.push_str("Validating flow state\n");
        response.push_str("Running deadlock check\n");
        response.push_str(&self.deadlock_check(state));

        response
    }

    /*
       Get ready to start execution (and debugging) from scratch at the start of the flow
    */
    fn reset(&mut self) {
        // Leave all the breakpoints untouched for the repeat run
        self.break_at_job = usize::MAX;
    }

    /*
       Take one step (execute one more job) in the flow. Do this by setting a breakpoint at the
       next job execution and then returning - flow execution will continue until breakpoint fires
    */
    fn step(&mut self, state: &RunState, steps: Option<Param>) {
        match steps {
            None => {
                self.break_at_job = state.jobs_created() + 1;
            }
            Some(Param::Numeric(steps)) => {
                self.break_at_job = state.jobs_created() + steps;
            }
            _ => self.debug_server.debugger_error(
                "Did not understand step command parameter\n".into()),
        }
    }

    /*
        Return a vector of all the processes preventing process_id from running, which can be:
        - other process has input full and hence is blocking running of this process
        - other process is the only process that sends to an empty input of this process
    */
    fn find_blockers(&self, state: &RunState, process_id: usize) -> Vec<BlockerNode> {
        let mut blockers: Vec<BlockerNode> = state
            .get_output_blockers(process_id)
            .iter()
            .map(|(id, _)| BlockerNode::new(*id, BlockType::OutputBlocked))
            .collect();

        let input_blockers: Vec<BlockerNode> = state
            .get_input_blockers(process_id)
            .iter()
            .map(|(id, _)| BlockerNode::new(*id, BlockType::UnreadySender))
            .collect();

        blockers.extend(input_blockers);

        blockers
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
    ) -> Vec<BlockerNode> {
        visited_nodes.push(node.function_id);
        node.blockers = self.find_blockers(state, node.function_id);

        for blocker in &mut node.blockers {
            if blocker.function_id == root_node_id {
                return vec![blocker.clone()]; // add the last node in the loop to end of trail
            }

            // if we've visited this blocking node before, then we've detected a loop
            if !visited_nodes.contains(&blocker.function_id) {
                let mut blocker_subtree =
                    self.traverse_blocker_tree(state, visited_nodes, root_node_id, blocker);
                if !blocker_subtree.is_empty() {
                    // insert this node at the head of the list of blocking nodes
                    blocker_subtree.insert(0, blocker.clone());
                    return blocker_subtree;
                }
            }
        }

        // no loop found
        vec![]
    }

    fn display_set(root_node: &BlockerNode, node_set: Vec<BlockerNode>) -> String {
        let mut display_string = String::new();
        display_string.push_str(&format!("#{}", root_node.function_id));
        for node in node_set {
            display_string.push_str(&format!("{}", node));
        }
        display_string
    }

    fn deadlock_check(&self, state: &RunState) -> String {
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
            );
            if !deadlock_set.is_empty() {
                response.push_str(&format!(
                    "{}\n",
                    Self::display_set(&root_node, deadlock_set)
                ));
            }
        }

        response
    }
}
