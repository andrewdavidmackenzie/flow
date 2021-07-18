use std::collections::HashSet;
use std::fmt;

use log::error;
use serde_json::Value;

use flowcore::output_connection::Source::{Input, Output};

use crate::client_server::ServerConnection;
use crate::debug_messages::DebugClientMessage;
use crate::debug_messages::DebugClientMessage::*;
use crate::debug_messages::DebugServerMessage;
use crate::debug_messages::DebugServerMessage::*;
use crate::debug_messages::Param;
use crate::errors::*;
use crate::run_state::{Block, Job, RunState};

/// Debugger struct contains all the info necessary to conduct a debugging session, storing
/// set breakpoints, connections to the debug client etc
pub struct Debugger {
    debug_server_connection: ServerConnection<DebugServerMessage, DebugClientMessage>,
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
    io_number: usize,
    block_type: BlockType,
    blockers: Vec<BlockerNode>,
}

impl BlockerNode {
    fn new(process_id: usize, io_number: usize, block_type: BlockType) -> Self {
        BlockerNode {
            function_id: process_id,
            io_number,
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

impl Debugger {
    pub fn new(
        debug_server_connection: ServerConnection<DebugServerMessage, DebugClientMessage>,
    ) -> Self {
        Debugger {
            debug_server_connection,
            input_breakpoints: HashSet::<(usize, usize)>::new(),
            block_breakpoints: HashSet::<(usize, usize)>::new(),
            output_breakpoints: HashSet::<(usize, String)>::new(),
            break_at_job: usize::MAX,
            function_breakpoints: HashSet::<usize>::new(),
        }
    }

    /// Start the debugger - preparing it to be entered later when needed
    pub fn start(&mut self) {
        let _ = self.debug_server_connection.start();
    }

    /// Enter the debugger - which will cause it to wait for a valid debugger command
    /// Return values are (display next output, reset execution, exit_debugger)
    pub fn enter(&mut self, state: &RunState) -> (bool, bool, bool) {
        self.wait_for_command(state)
    }

    /// Called from the flowrlib coordinator to inform the debugger that a job has completed
    /// being executed. It is used to inform the debug client of the fact.
    /// Return values are (display next output, reset execution)
    pub fn job_completed(&mut self, job: &Job) -> (bool, bool, bool) {
        let _: Result<DebugClientMessage> = self.debug_server_connection.send_message(
            JobCompleted(job.job_id, job.function_id, job.result.0.clone()),
        );
        (false, false, false)
    }

    /// Check if there is a breakpoint at this job prior to starting executing it.
    /// Return values are (display next output, reset execution)
    pub fn check_prior_to_job(
        &mut self,
        state: &RunState,
        next_job_id: usize,
        function_id: usize,
    ) -> (bool, bool, bool) {
        if self.break_at_job == next_job_id || self.function_breakpoints.contains(&function_id) {
            let _: Result<DebugClientMessage> = self
                .debug_server_connection
                .send_message(PriorToSendingJob(next_job_id, function_id));

            // display the status of the function we stopped prior to executing
            let event = DebugServerMessage::FunctionState((
                state.get(function_id).clone(),
                state.get_state(function_id),
            ));
            let _: Result<DebugClientMessage> = self.debug_server_connection.send_message(event);

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
            let _: Result<DebugClientMessage> = self
                .debug_server_connection
                .send_message(BlockBreakpoint(block.clone()));
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
        source_process_id: usize,
        output_route: &str,
        value: &Value,
        destination_id: usize,
        input_number: usize,
    ) -> (bool, bool, bool) {
        if self
            .output_breakpoints
            .contains(&(source_process_id, output_route.to_string()))
            || self
                .input_breakpoints
                .contains(&(destination_id, input_number))
        {
            let _: Result<DebugClientMessage> =
                self.debug_server_connection.send_message(SendingValue(
                    source_process_id,
                    value.clone(),
                    destination_id,
                    input_number,
                ));
            let _: Result<DebugClientMessage> =
                self.debug_server_connection.send_message(DataBreakpoint(
                    source_process_id,
                    output_route.to_string(),
                    value.clone(),
                    destination_id,
                    input_number,
                ));
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
    pub fn job_error(&mut self, state: &RunState, job: Job) -> (bool, bool, bool) {
        let _: Result<DebugClientMessage> =
            self.debug_server_connection.send_message(JobError(job));
        self.wait_for_command(state)
    }

    /// A panic occurred while executing a flow. Let the debug client know, enter the client
    /// and wait for a user command.
    ///
    /// This is useful for debugging a flow that has an error. Without setting any explicit
    /// breakpoint it will enter the debugger on an error and let the user inspect the flow's
    /// state etc.
    pub fn panic(&mut self, state: &RunState, error_message: String) -> (bool, bool, bool) {
        let _: Result<DebugClientMessage> = self
            .debug_server_connection
            .send_message(Panic(error_message, state.jobs_created()));
        self.wait_for_command(state)
    }

    /// Return values are (display next output, reset execution)
    pub fn flow_done(&mut self, state: &RunState) -> (bool, bool, bool) {
        let _: Result<DebugClientMessage> =
            self.debug_server_connection.send_message(ExecutionEnded);
        self.deadlock_check(state);
        self.wait_for_command(state)
    }

    /*
        The execution flow has entered the debugged based on some event.

        Now wait for and process commands from the DebugClient
           - execute and respond immediately those that require it
           - some commands will cause the command loop to exit.

        When exiting return a tuple for the Coordinator to determine what to do:
           (display next output, reset execution)
    */
    fn wait_for_command(&mut self, state: &RunState) -> (bool, bool, bool) {
        // swallow the first DebugClientStarting message to initialize the connection
        let _ = self.debug_server_connection.get_message();

        loop {
            match self
                .debug_server_connection
                .send_message(WaitingForCommand(state.jobs_created()))
            {
                // *************************      The following are commands that send a response
                Ok(Breakpoint(param)) => {
                    let event = self.add_breakpoint(state, param);
                    let _ = self.debug_server_connection.send_message(event);
                }
                Ok(Delete(param)) => {
                    let event = self.delete_breakpoint(param);
                    let _ = self.debug_server_connection.send_message(event);
                }
                Ok(Validate) => {
                    let event = self.validate(state);
                    let _ = self.debug_server_connection.send_message(event);
                }
                Ok(List) => {
                    let event = self.list_breakpoints();
                    let _ = self.debug_server_connection.send_message(event);
                }
                Ok(Inspect) => {
                    let _ = self
                        .debug_server_connection
                        .send_message(DebugServerMessage::OverallState(state.clone()));
                }
                Ok(InspectFunction(function_id)) => {
                    let event = if function_id < state.num_functions() {
                        DebugServerMessage::FunctionState((
                            state.get(function_id).clone(),
                            state.get_state(function_id),
                        ))
                    } else {
                        DebugServerMessage::Error(format!("No function with id = {}", function_id))
                    };

                    let _: Result<DebugClientMessage> =
                        self.debug_server_connection.send_message(event);
                }
                Ok(InspectInput(function_id, input_number)) => {
                    let event = if function_id < state.num_functions() {
                        let function = state.get(function_id);

                        if input_number < function.inputs().len() {
                            DebugServerMessage::InputState(function.input(input_number).clone())
                        } else {
                            DebugServerMessage::Error(format!(
                                "Function #{} has no input number {}",
                                function_id, input_number
                            ))
                        }
                    } else {
                        DebugServerMessage::Error(format!("No function with id = {}", function_id))
                    };
                    let _: Result<DebugClientMessage> =
                        self.debug_server_connection.send_message(event);
                }
                Ok(InspectOutput(function_id, sub_route)) => {
                    let event = if function_id < state.num_functions() {
                        let function = state.get(function_id);

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
                        DebugServerMessage::OutputState(output_connections)
                    } else {
                        DebugServerMessage::Error(format!("No function with id = {}", function_id))
                    };
                    let _: Result<DebugClientMessage> =
                        self.debug_server_connection.send_message(event);
                }
                Ok(InspectBlock(from_function_id, to_function_id)) => {
                    let event = Self::inspect_blocks(state, from_function_id, to_function_id);
                    let _ = self.debug_server_connection.send_message(event);
                }
                Ok(ExitDebugger) => {
                    let _: Result<DebugClientMessage> =
                        self.debug_server_connection.send_message(ExitingDebugger);
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
                        let event = self.reset();
                        let _: Result<DebugClientMessage> =
                            self.debug_server_connection.send_message(event);
                        (false, true, false)
                    } else {
                        let _: Result<DebugClientMessage> =
                            self.debug_server_connection.send_message(ExecutionStarted);
                        (false, false, false)
                    }
                }
                Ok(Step(param)) => {
                    let _ = self.step(state, param);
                    return (true, false, false);
                }
                Ok(Ack) => {}
                Ok(DebugClientStarting) => {
                    error!("Unexpected message 'DebugClientStarting' after started")
                }
                Ok(DebugClientMessage::Error(_)) => { /* client error */ }
                Ok(DebugClientMessage::Invalid) => {}
                Err(e) => error!("Error in Debug server getting command; {}", e),
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
    ) -> DebugServerMessage {
        let mut matching_blocks = vec![];

        for block in run_state.get_blocks() {
            if (from.is_none() || from == Some(block.blocked_id))
                && (to.is_none() || to == Some(block.blocking_id))
            {
                matching_blocks.push(block.clone());
            }
        }

        DebugServerMessage::BlockState(matching_blocks)
    }

    /****************************** Implementations of Debugger Commands *************************/

    /*
       Add a breakpoint to the debugger according to the Optional `Param`
    */
    fn add_breakpoint(&mut self, state: &RunState, param: Option<Param>) -> DebugServerMessage {
        let mut response = String::new();

        match param {
            None => response.push_str("'break' command must specify a breakpoint\n"),
            Some(Param::Numeric(process_id)) => {
                if process_id > state.num_functions() {
                    response.push_str(&format!(
                        "There is no process with id '{}' to set a breakpoint on\n",
                        process_id
                    ));
                } else {
                    self.function_breakpoints.insert(process_id);
                    response.push_str(&format!(
                        "Set process breakpoint on Function #{}\n",
                        process_id
                    ));
                }
            }
            Some(Param::Input((destination_id, input_number))) => {
                response.push_str(&format!(
                    "Set data breakpoint on process #{} receiving data on input: {}\n",
                    destination_id, input_number
                ));
                self.input_breakpoints
                    .insert((destination_id, input_number));
            }
            Some(Param::Block((Some(blocked_id), Some(blocking_id)))) => {
                response.push_str(&format!(
                    "Set block breakpoint for Function #{} being blocked by Function #{}\n",
                    blocked_id, blocking_id
                ));
                self.block_breakpoints.insert((blocked_id, blocking_id));
            }
            Some(Param::Block(_)) => {
                response.push_str("Invalid format to set a breakpoint on a block\n");
            }
            Some(Param::Output((source_id, source_output_route))) => {
                response.push_str(&format!(
                    "Set data breakpoint on process #{} sending data via output: '/{}'\n",
                    source_id, source_output_route
                ));
                self.output_breakpoints
                    .insert((source_id, source_output_route));
            }
            Some(Param::Wildcard) => {
                response.push_str(
                    "To break on every process, you can just single step using 's' command\n",
                );
            }
        }

        Message(response)
    }

    /*
       Delete debugger breakpoints related to Jobs or Blocks, etc according to the Spec.
    */
    fn delete_breakpoint(&mut self, param: Option<Param>) -> DebugServerMessage {
        let mut response = String::new();

        match param {
            None => response.push_str("No process id specified\n"),
            Some(Param::Numeric(process_number)) => {
                if self.function_breakpoints.remove(&process_number) {
                    response.push_str(&format!(
                        "Breakpoint on process #{} was deleted\n",
                        process_number
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

        Message(response)
    }

    /*
       List all debugger breakpoints of all types.
    */
    fn list_breakpoints(&self) -> DebugServerMessage {
        let mut response = String::new();

        let mut breakpoints = false;
        if !self.function_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Function Breakpoints: \n");
            for process_id in &self.function_breakpoints {
                response.push_str(&format!("\tFunction #{}\n", process_id));
            }
        }

        if !self.output_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Output Breakpoints: \n");
            for (process_id, route) in &self.output_breakpoints {
                response.push_str(&format!("\tOutput #{}/{}\n", process_id, route));
            }
        }

        if !self.input_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Input Breakpoints: \n");
            for (process_id, input_number) in &self.input_breakpoints {
                response.push_str(&format!("\tInput #{}:{}\n", process_id, input_number));
            }
        }

        if !self.block_breakpoints.is_empty() {
            breakpoints = true;
            response.push_str("Block Breakpoints: \n");
            for (blocked_id, blocking_id) in &self.block_breakpoints {
                response.push_str(&format!("\tBlock #{}->#{}\n", blocked_id, blocking_id));
            }
        }

        if !breakpoints {
            response.push_str(
                "No Breakpoints set. Use the 'b' command to set a breakpoint. Use 'h' for help.\n",
            );
        }

        Message(response)
    }

    /*
       Run checks on the current flow execution state to check if it is valid
       Currently deadlock check is the only check that exists.
    */
    fn validate(&self, state: &RunState) -> DebugServerMessage {
        let mut response = String::new();

        response.push_str("Validating flow state\n");
        response.push_str("Running deadlock check\n");
        response.push_str(&self.deadlock_check(state));

        Message(response)
    }

    /*
       Get ready to start execution (and debugging) from scratch at the start of the flow
    */
    fn reset(&mut self) -> DebugServerMessage {
        // Leave all the breakpoints untouched for the repeat run
        self.break_at_job = usize::MAX;
        Resetting
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
            _ => {
                let _ = self
                    .debug_server_connection
                    .send_message(DebugServerMessage::Error(
                        "Did not understand step command parameter\n".into(),
                    ));
            }
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
            .map(|(id, io)| BlockerNode::new(*id, *io, BlockType::OutputBlocked))
            .collect();

        let input_blockers: Vec<BlockerNode> = state
            .get_input_blockers(process_id)
            .iter()
            .map(|(id, io)| BlockerNode::new(*id, *io, BlockType::UnreadySender))
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
            let mut root_node = BlockerNode::new(*blocked_process_id, 0, BlockType::OutputBlocked);
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
