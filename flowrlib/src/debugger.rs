use std::collections::HashSet;
use std::fmt;
use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use log::debug;
use serde_json::Value;

use crate::debug_client::Command::{*};
use crate::debug_client::DebugClient;
use crate::debug_client::Event::{*};
use crate::debug_client::Param;
use crate::debug_client::Response;
use crate::debug_client::Response::{*};
use crate::run_state::{Block, Job, RunState};

pub struct Debugger {
    client: &'static dyn DebugClient,
    input_breakpoints: HashSet<(usize, usize)>,
    block_breakpoints: HashSet<(usize, usize)>,
    /* blocked_id -> blocking_id */
    output_breakpoints: HashSet<(usize, String)>,
    break_at_job: usize,
    function_breakpoints: HashSet<usize>,
    debug_requested: Arc<AtomicBool>
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
    blocktype: BlockType,
    blockers: Vec<BlockerNode>,
}

impl BlockerNode {
    fn new(process_id: usize, io_number: usize, blocktype: BlockType) -> Self {
        BlockerNode {
            function_id: process_id,
            io_number,
            blocktype,
            blockers: vec!(),
        }
    }
}

impl fmt::Display for BlockerNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.blocktype {
            BlockType::OutputBlocked => write!(f, " -> #{}", self.function_id),
            BlockType::UnreadySender => write!(f, " <- #{}", self.function_id)
        }
    }
}

impl Debugger {
    pub fn new(client: &'static dyn DebugClient) -> Self {
        Debugger {
            client,
            input_breakpoints: HashSet::<(usize, usize)>::new(),
            block_breakpoints: HashSet::<(usize, usize)>::new(),
            output_breakpoints: HashSet::<(usize, String)>::new(),
            break_at_job: std::usize::MAX,
            function_breakpoints: HashSet::<usize>::new(),
            debug_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    /*
        Initialize the debugger. This does not enter the debugger it just prepares the Control-C
        capture to later be able to enter the debugger from it.
     */
    pub fn init(&mut self) {
        self.capture_control_c();
        debug!("Control-C capture setup to enter debugger");
    }

    /*
        Return values are (display next output, reset execution)
    */
    pub fn enter(&mut self, state: &RunState) -> (bool, bool) {
        self.client.send_event(Enter);
        self.wait_for_command(state)
    }

    /*
        Called from the flowrlib coordinator to inform the debugger that a job has completed
        being executed. It is used to inform the debug client of the fact.
        Return values are (display next output, reset execution)
    */
    pub fn job_completed(&mut self, state: &RunState, job: &Job) -> (bool, bool) {
        self.client.send_event(JobCompleted(job.job_id, job.function_id, job.result.0.clone()));

        if self.debug_requested.load(Ordering::SeqCst) {
            self.wait_for_command(state)
        } else {
            (false, false)
        }
    }

    /*
        Check if there is a breakpoint at this job prior to starting executing it.
        Return values are (display next output, reset execution)
    */
    pub fn check_prior_to_job(&mut self, state: &RunState, next_job_id: usize, function_id: usize) -> (bool, bool) {
        if self.break_at_job == next_job_id ||
            self.function_breakpoints.contains(&function_id) {
            self.client.send_event(PriorToSendingJob(next_job_id, function_id));
            self.print(state, Some(Param::Numeric(function_id)));
            return self.wait_for_command(state);
        }

        if self.debug_requested.load(Ordering::SeqCst) {
            self.wait_for_command(state)
        } else {
            // No breakpoint - continue execution
            (false, false)
        }
    }

    /*
        Called from flowrlib during execution when it is about to create a block on one function
        due to not being able to send outputs to another function.

        This allows the debugger to check if we have a breakpoint set on that block. If we do
        then enter the debugger client and wait for a command.
    */
    pub fn check_on_block_creation(&mut self, state: &RunState, block: &Block) {
        if self.block_breakpoints.contains(&(block.blocked_id, block.blocking_id)) {
            self.client.send_event(BlockBreakpoint(block.clone()));
            self.wait_for_command(state);
        }

        if self.debug_requested.load(Ordering::SeqCst) {
            self.wait_for_command(state);
        }
    }

    /*
        Called from flowrlib runtime prior to sending a value to another function,to see if there
        is a breakpoint on that send.

        If there is, then enter the debug client and wait for a command.
    */
    pub fn check_prior_to_send(&mut self, state: &RunState, source_process_id: usize, output_route: &str,
                               value: &Value, destination_id: usize, input_number: usize) {
        if self.output_breakpoints.contains(&(source_process_id, output_route.to_string())) ||
            self.input_breakpoints.contains(&(destination_id, input_number)) {
            self.client.send_event(SendingValue(
                source_process_id, value.clone(), destination_id, input_number));
            self.client.send_event(DataBreakpoint(source_process_id, output_route.to_string(),
                                                  value.clone(), destination_id, input_number));
            self.wait_for_command(state);
        }

        if self.debug_requested.load(Ordering::SeqCst) {
            self.wait_for_command(state);
        }
    }

    /*
        An error occurred while executing a flow. Let the debug client know, enter the client
        and wait for a user command.

        This is useful for debugging a flow that has an error. Without setting any explicit
        breakpoint it will enter the debugger on an error and let the user inspect the flow's
        state etc.
    */
    pub fn error(&mut self, state: &RunState, job: Job) {
        self.client.send_event(JobError(job));
        self.wait_for_command(state);
    }

    /*
        A panic occurred while executing a flow. Let the debug client know, enter the client
        and wait for a user command.

        This is useful for debugging a flow that has an error. Without setting any explicit
        breakpoint it will enter the debugger on an error and let the user inspect the flow's
        state etc.
    */
    pub fn panic(&mut self, state: &RunState, error_message: String) {
        self.client.send_event(Panic(error_message));
        self.wait_for_command(state);
    }

    /*
        Return values are (display next output, reset execution)
    */
    pub fn end(&mut self, state: &RunState) -> (bool, bool) {
        self.client.send_event(End);
        self.deadlock_inspection(state);
        self.wait_for_command(state)
    }

    /*
        Setup a control-c signal capture. When captured set the debug_requested flag to true.
        That is checked in calls to the debugger on certain runtime events. When the true is
        detected we will enter the debugger command line
     */
    fn capture_control_c(&mut self) {
        let requested = self.debug_requested.clone();

        ctrlc::set_handler(move || {
            requested.store(true, Ordering::SeqCst);
        }).expect("Error setting Ctrl-C handler");
    }

    /*
        The execution flow has entered the debugged based on some event.

        Now wait for and process commands from the DebugClient
           - execute and respond immediately those that require it
           - some commands will cause the command loop to exit.

        When exiting return a tuple for the Coordinaator to determine what to do:
           (display next output, reset execution)
    */
    fn wait_for_command(&mut self, state: &RunState) -> (bool, bool) {
        loop {
            match self.client.get_command(state.jobs_sent()) {
                // *************************      The following are commands that send a response
                GetState => {
                    // Respond with 'state'
                }
                GetFunctionState(_id) => {
                    // Respond with display_state(&self, function_id: usize) -> String ??
                }
                Breakpoint(param) =>
                    self.client.send_response(self.add_breakpoint(state, param)),
                Delete(param) =>
                    self.client.send_response(self.delete_breakpoint(param)),
                Inspect =>
                    self.client.send_response(self.inspect(state)),
                List =>
                    self.client.send_response(self.list_breakpoints()),
                Print(param) =>
                    self.client.send_response(self.print(state, param)),
                EnterDebugger => { /* Not needed as we are already in the debugger */ }
                ExitDebugger => {
                    self.client.send_response(Exiting);
                    exit(1);
                }

                // **************************      The following commands exit the command loop
                Continue => {
                    if state.jobs_sent() > 0 {
                        self.client.send_response(Ack);
                        return (false, false);
                    }
                }
                RunReset => {
                    if state.jobs_sent() > 0 {
                        self.client.send_response(self.reset());
                        return (false, true);
                    } else {
                        self.client.send_response(Running);
                        return (false, false);
                    }
                }
                Step(param) => {
                    self.client.send_response(self.step(state, param));
                    return (true, false);
                }
            };
        }
    }

    /****************************** Implementations of Debugger Commands *************************/

    /*
        Add a breakpoint to the debugger according to the Optional `Param`
     */
    fn add_breakpoint(&mut self, state: &RunState, param: Option<Param>) -> Response {
        let mut response = String::new();

        match param {
            None => response.push_str("'break' command must specify a breakpoint\n"),
            Some(Param::Numeric(process_id)) => {
                if process_id > state.num_functions() {
                    response.push_str(&format!("There is no process with id '{}' to set a breakpoint on\n",
                                               process_id));
                } else {
                    self.function_breakpoints.insert(process_id);
                    response.push_str(&format!("Set process breakpoint on Function #{}\n",
                                               process_id));
                }
            }
            Some(Param::Input((dest_id, input_number))) => {
                response.push_str(&format!("Set data breakpoint on process #{} receiving data on input: {}\n",
                                           dest_id, input_number));
                self.input_breakpoints.insert((dest_id, input_number));
            }
            Some(Param::Block((blocked_id, blocking_id))) => {
                response.push_str(&format!("Set block breakpoint for Function #{} being blocked by Function #{}\n",
                                           blocked_id, blocking_id));
                self.block_breakpoints.insert((blocked_id, blocking_id));
            }
            Some(Param::Output((source_id, source_output_route))) => {
                response.push_str(&format!("Set data breakpoint on process #{} sending data via output: '/{}'\n",
                                           source_id, source_output_route));
                self.output_breakpoints.insert((source_id, source_output_route));
            }
            Some(Param::Wildcard) => {
                response.push_str("To break on every process, you can just single step using 's' command\n");
            }
        }

        Message(response)
    }

    /*
        Delete debugger breakpoints related to Jobs or Blocks, etc according to the Spec.
     */
    fn delete_breakpoint(&mut self, param: Option<Param>) -> Response {
        let mut response = String::new();

        match param {
            None => response.push_str("No process id specified\n"),
            Some(Param::Numeric(process_number)) => {
                if self.function_breakpoints.remove(&process_number) {
                    response.push_str(&format!("Breakpoint on process #{} was deleted\n", process_number));
                } else {
                    response.push_str("No breakpoint number '{}' exists\n");
                }
            }
            Some(Param::Input((dest_id, input_number))) => {
                self.input_breakpoints.remove(&(dest_id, input_number));
                response.push_str("Inputs breakpoint removed\n");
            }
            Some(Param::Block((blocked_id, blocking_id))) => {
                self.input_breakpoints.remove(&(blocked_id, blocking_id));
                response.push_str("Inputs breakpoint removed\n");
            }
            Some(Param::Output((source_id, source_output_route))) => {
                self.output_breakpoints.remove(&(source_id, source_output_route));
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
    fn list_breakpoints(&self) -> Response {
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
            response.push_str("No Breakpoints set. Use the 'b' command to set a breakpoint. Use 'h' for help.\n");
        }

        Message(response)
    }

    /*
        Run inspections on the current flow execution state to possibly detect issues.
        Currently deadlock inspection is the only inspection that exists.s
     */
    fn inspect(&self, state: &RunState) -> Response {
        let mut response = String::new();

        response.push_str("Running inspections\n");
        response.push_str("Running deadlock inspection\n");
        response.push_str(&self.deadlock_inspection(state));

        Message(response)
    }

    /*
        Print out a debugger value according to the Optional `Param` passed.
        If no `Param` is passed then print the entire state
     */
    fn print(&self, state: &RunState, param: Option<Param>) -> Response {
        let mut response = String::new();

        match param {
            None => response.push_str(&format!("{}\n", state)),
            Some(Param::Numeric(function_id)) |
            Some(Param::Block((function_id, _))) => {
                response.push_str(&self.print_function(state, function_id));
            }
            Some(Param::Input((function_id, _))) => {
                response.push_str(&self.print_function(state, function_id));
            }
            Some(Param::Wildcard) => {
                response.push_str(&self.print_all_processes(state))
            }
            Some(Param::Output(_)) => response.push_str(
                "Cannot display the output of a process until it is executed. \
                Set a breakpoint on the process by id and then step over it")
        }

        Message(response)
    }

    /*
        Get ready to start execution (and debugging) from scratch at the start of the flow
     */
    fn reset(&mut self) -> Response {
        // Leave all the breaakpoints untouch for the repeat run
        self.break_at_job = std::usize::MAX;
        Resetting
    }

    /*
        Take one step (execute one more job) in the flow. Do this by setting a breakpoint at the
        next job execution and then returning - flow execution will continue until breakpoint fires
     */
    fn step(&mut self, state: &RunState, steps: Option<Param>) -> Response {
        match steps {
            None => {
                self.break_at_job = state.jobs() + 1;
                Ack
            }
            Some(Param::Numeric(steps)) => {
                self.break_at_job = state.jobs() + steps;
                Ack
            }
            _ => {
                Error("Did not understand step command parameter\n".into())
            }
        }
    }

    /*
        Return a vector of all the processes preventing process_id from running, which can be:
        - other process has input full and hence is blocking running of this process
        - other process is the only process that sends to an empty input of this process
    */
    fn find_blockers(&self, state: &RunState, process_id: usize) -> Vec<BlockerNode> {
        let mut blockers: Vec<BlockerNode> = state.get_output_blockers(process_id).iter().map(|(id, io)|
            BlockerNode::new(*id, *io, BlockType::OutputBlocked)).collect();

        let input_blockers: Vec<BlockerNode> = state.get_input_blockers(process_id).iter().map(|(id, io)|
            BlockerNode::new(*id, *io, BlockType::UnreadySender)).collect();

        blockers.extend(input_blockers);

        blockers
    }

    /*
        Traverse the tree of processes blocking this process from running, either because:
        - this process wants to send to the other, but the input it full
        - this process needs an input from the other

        Return true if a loop was detected, false if done without detecting a loop
    */
    fn traverse_blocker_tree(&self, state: &RunState, visited_nodes: &mut Vec<usize>,
                             root_node_id: usize, node: &mut BlockerNode) -> Vec<BlockerNode> {
        visited_nodes.push(node.function_id);
        node.blockers = self.find_blockers(state, node.function_id);

        for blocker in &mut node.blockers {
            if blocker.function_id == root_node_id {
                return vec!(blocker.clone()); // add the last node in the loop to end of trail
            }

            // if we've visited this blocking node before, then we've detected a loop
            if !visited_nodes.contains(&blocker.function_id) {
                let mut blocker_subtree = self.traverse_blocker_tree(state, visited_nodes,
                                                                     root_node_id, blocker);
                if !blocker_subtree.is_empty() {
                    // insert this node at the head of the list of blocking nodes
                    blocker_subtree.insert(0, blocker.clone());
                    return blocker_subtree;
                }
            }
        }

        // no loop found
        vec!()
    }

    fn display_set(root_node: &BlockerNode, node_set: Vec<BlockerNode>) -> String {
        let mut display_string = String::new();
        display_string.push_str(&format!("#{}", root_node.function_id));
        for node in node_set {
            display_string.push_str(&format!("{}", node));
        }
        display_string
    }

    fn deadlock_inspection(&self, state: &RunState) -> String {
        let mut response = String::new();

        for blocked_process_id in state.get_blocked() {
            // start a clean tree with a new root node for each blocked process
            let mut root_node = BlockerNode::new(*blocked_process_id, 0, BlockType::OutputBlocked);
            let mut visited_nodes = vec!();

            let deadlock_set = self.traverse_blocker_tree(state, &mut visited_nodes,
                                                          *blocked_process_id, &mut root_node);
            if !deadlock_set.is_empty() {
                response.push_str(&format!("{}\n", Self::display_set(&root_node, deadlock_set)));
            }
        }

        response
    }

    fn print_function(&self, state: &RunState, function_id: usize) -> String {
        let mut response = String::new();

        let function = state.get(function_id);
        response.push_str(&format!("{}", function));
        response.push_str(&state.display_state(function_id));

        response
    }

    fn print_all_processes(&self, state: &RunState) -> String {
        let mut response = String::new();

        for id in 0..state.num_functions() {
            response.push_str(&self.print_function(state, id));
        }

        response
    }
}
