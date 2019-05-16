use debug_client::DebugClient;
use std::process::exit;
use run_state::RunState;
use std::collections::HashSet;
use serde_json::Value;
use std::fmt;

pub struct Debugger {
    pub client: &'static DebugClient,
    input_breakpoints: HashSet<(usize, usize)>,
    block_breakpoints: HashSet<(usize, usize)>,
    /* blocked_id -> blocking_id */
    output_breakpoints: HashSet<(usize, String)>,
    break_at_job: usize,
    function_breakpoints: HashSet<usize>,
}

const HELP_STRING: &str = "Debugger commands:
'b' | 'breakpoint' {spec}    - Set a breakpoint on a process, an output or an input using spec:
                                - function_id
                                - source_id/output_route ('source_id/' for default output route)
                                - destination_id:input_number
                                - blocked_process_id->blocking_process_id
ENTER | 'c' | 'continue'     - Continue execution until next breakpoint
'd' | 'delete' {spec} or '*' - Delete the breakpoint matching {spec} or all with '*'
'e' | 'exit'                 - Stop flow execution and exit debugger
'h' | 'help'                 - Display this help message
'i' | 'inspect'              - Run a series of defined 'inspections' to check status of flow
'l' | 'list'                 - List all breakpoints
'p' | 'print' [n]            - Print the overall state, or state of process number 'n'
'r' | 'reset'                - reset the state back to initial state after loading
's' | 'step' [n]             - Step over the next 'n' jobs (default = 1) then break
'q' | 'quit'                 - Stop flow execution and exit debugger
";

enum Param {
    Wildcard,
    Numeric(usize),
    Output((usize, String)),
    Input((usize, usize)),
    Block((usize, usize)),
}

#[derive(Debug, Clone)]
enum BlockType {
    OutputBlocked,
    // Cannot run and send it's Output as a destination Input is full
    UnreadySender,  // Has to send output to an empty Input for other process to be able to run
}

#[derive(Debug, Clone)]
struct BlockerNode {
    process_id: usize,
    io_number: usize,
    blocktype: BlockType,
    blockers: Vec<BlockerNode>,
}

impl BlockerNode {
    fn new(process_id: usize, io_number: usize, blocktype: BlockType) -> Self {
        BlockerNode {
            process_id,
            io_number,
            blocktype,
            blockers: vec!(),
        }
    }
}

impl fmt::Display for BlockerNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.blocktype {
            BlockType::OutputBlocked => write!(f, " -> #{}", self.process_id),
            BlockType::UnreadySender => write!(f, " <- #{}", self.process_id)
        }
    }
}

impl Debugger {
    pub fn new(client: &'static DebugClient) -> Self {
        Debugger {
            client,
            input_breakpoints: HashSet::<(usize, usize)>::new(),
            block_breakpoints: HashSet::<(usize, usize)>::new(),
            output_breakpoints: HashSet::<(usize, String)>::new(),
            break_at_job: 0,
            function_breakpoints: HashSet::<usize>::new(),
        }
    }

    /*
        Return values are (display next output, reset execution)
    */
    pub fn start(&mut self, state: &mut RunState) -> (bool, bool) {
        self.client.display("Entering Debugger:\n");
        return self.command_loop(state);
    }

    /*
        Return values are (display next output, reset execution)
    */
    pub fn check(&mut self, state: &mut RunState, next_job_id: usize, function_id: usize) -> (bool, bool) {
        if self.break_at_job == next_job_id ||
            self.function_breakpoints.contains(&function_id) {
            self.client.display(&format!("Sending Job #{}:\n", next_job_id));
            self.print(state, Some(Param::Numeric(function_id)));
            return self.command_loop(state);
        }

        (false, false)
    }

    pub fn check_block(&mut self, state: &mut RunState, blocking_id: usize,
                       blocking_io_number: usize, blocked_id: usize) {
        if self.block_breakpoints.contains(&(blocked_id, blocking_id)) {
            self.client.display(&format!("Block breakpoint: Function #{} ----- blocked by ----> Function #{}:{}\n",
                                         blocked_id, blocking_id, blocking_io_number));
            self.command_loop(state);
        }
    }

    pub fn watch_data(&mut self, state: &mut RunState, source_process_id: usize, output_route: &String,
                      value: &Value, destination_id: usize, input_number: usize) {
        if self.output_breakpoints.contains(&(source_process_id, output_route.to_string())) ||
            self.input_breakpoints.contains(&(destination_id, input_number)) {
            self.client.display(&format!("Data breakpoint: Function #{}{}    ----- {} ----> Function #{}:{}\n",
                                         source_process_id, output_route, value,
                                         destination_id, input_number));
            self.command_loop(state);
        }
    }

    pub fn panic(&mut self, state: &mut RunState, id: usize, inputs: &Vec<Vec<Value>>) {
        self.client.display("Entering debugger\n");
        self.client.display(&format!("Function #{} ran with inputs: {:?}\n", id, inputs.clone()));
        self.command_loop(state);
    }

    /*
        Return values are (display next output, reset execution)
    */
    pub fn end(&mut self, state: &mut RunState) -> (bool, bool) {
        self.client.display("Execution has ended\n");
        self.deadlock_inspection(state);
        let (display, restart) = self.command_loop(state);
        if !restart {
            self.client.display("Exiting debugger\n");
        }
        (display, restart)
    }

    /*
        Return values are (display next output, reset execution)
    */
    pub fn command_loop(&mut self, state: &mut RunState) -> (bool, bool) {
        loop {
            self.client.display(&format!("Debug #{}> ", state.jobs()));
            let mut input = String::new();
            match self.client.read_input(&mut input) {
                Ok(_n) => {
                    let (command, param) = Self::parse_command(&input);
                    match command {
                        "b" | "breakpoint" => self.add_breakpoint(state, param),
                        "" | "c" | "continue" => return (false, false),
                        "d" | "delete" => self.delete_breakpoint(param),
                        "e" | "exit" => exit(1),
                        "h" | "help" => self.help(),
                        "i" | "inspect" => self.inspect(state),
                        "l" | "list" => self.list_breakpoints(),
                        "p" | "print" => self.print(state, param),
                        "r" | "reset" => {
                            self.break_at_job = 0;
                            self.client.display("Resetting state\n");
                            state.reset();
                            return (false, true);
                        }
                        "s" | "step" => {
                            self.step(state, param);
                            return (true, false);
                        }
                        "q" | "quit" => exit(1),
                        _ => self.client.display(&format!("Unknown debugger command '{}'\n", command))
                    }
                }
                Err(_) => self.client.display(&format!("Error reading debugger command\n"))
            };
        }
    }

    fn add_breakpoint(&mut self, state: &RunState, param: Option<Param>) {
        match param {
            None => self.client.display("'break' command must specify a process id to break on"),
            Some(Param::Numeric(process_id)) => {
                if process_id > state.num_functions() {
                    self.client.display(
                        &format!("There is no process with id '{}' to set a breakpoint on\n", process_id));
                } else {
                    self.function_breakpoints.insert(process_id);
                    self.client.display(
                        &format!("Set process breakpoint on Function #{}\n", process_id));
                }
            }
            Some(Param::Input((dest_id, input_number))) => {
                self.client.display(
                    &format!("Set data breakpoint on process #{} receiving data on input: {}\n", dest_id, input_number));
                self.input_breakpoints.insert((dest_id, input_number));
            }
            Some(Param::Block((blocked_id, blocking_id))) => {
                self.client.display(
                    &format!("Set block breakpoint for Function #{} being blocked by Function #{}\n", blocked_id, blocking_id));
                self.block_breakpoints.insert((blocked_id, blocking_id));
            }
            Some(Param::Output((source_id, source_output_route))) => {
                self.client.display(
                    &format!("Set data breakpoint on process #{} sending data via output: '/{}'\n", source_id, source_output_route));
                self.output_breakpoints.insert((source_id, source_output_route));
            }
            Some(Param::Wildcard) => self.client.display("To break on every process, you can just single step using 's' command\n")
        }
    }

    fn delete_breakpoint(&mut self, param: Option<Param>) {
        match param {
            None => self.client.display("No process id specified\n"),
            Some(Param::Numeric(process_number)) => {
                if self.function_breakpoints.remove(&process_number) {
                    self.client.display(
                        &format!("Breakpoint on process #{} was deleted\n", process_number));
                } else {
                    self.client.display("No breakpoint number '{}' exists\n");
                }
            }
            Some(Param::Input((dest_id, input_number))) => {
                self.input_breakpoints.remove(&(dest_id, input_number));
            }
            Some(Param::Block((blocked_id, blocking_id))) => {
                self.input_breakpoints.remove(&(blocked_id, blocking_id));
            }
            Some(Param::Output((source_id, source_output_route))) => {
                self.output_breakpoints.remove(&(source_id, source_output_route));
            }
            Some(Param::Wildcard) => {
                self.output_breakpoints.clear();
                self.input_breakpoints.clear();
                self.function_breakpoints.clear();
                self.client.display("Deleted all breakpoints\n");
            }
        }
    }

    fn list_breakpoints(&self) {
        let mut breakpoints = false;
        if !self.function_breakpoints.is_empty() {
            breakpoints = true;
            self.client.display("Function Breakpoints: \n");
            for process_id in &self.function_breakpoints {
                self.client.display(&format!("\tFunction #{}\n", process_id));
            }
        }

        if !self.output_breakpoints.is_empty() {
            breakpoints = true;
            self.client.display("Output Breakpoints: \n");
            for (process_id, route) in &self.output_breakpoints {
                self.client.display(&format!("\tOutput #{}/{}\n", process_id, route));
            }
        }

        if !self.input_breakpoints.is_empty() {
            breakpoints = true;
            self.client.display("Input Breakpoints: \n");
            for (process_id, input_number) in &self.input_breakpoints {
                self.client.display(&format!("\tInput #{}:{}\n", process_id, input_number));
            }
        }

        if !self.block_breakpoints.is_empty() {
            breakpoints = true;
            self.client.display("Block Breakpoints: \n");
            for (blocked_id, blocking_id) in &self.block_breakpoints {
                self.client.display(&format!("\tBlock #{}->#{}\n", blocked_id, blocking_id));
            }
        }

        if !breakpoints {
            self.client.display("No Breakpoints set. Use the 'b' command to set a breakpoint. Use 'h' for help.\n");
        }
    }

    fn help(&self) {
        self.client.display(HELP_STRING);
    }

    fn inspect(&self, state: &RunState) {
        self.client.display("Running inspections\n");
        self.client.display("Running deadlock inspection\n");
        self.deadlock_inspection(state);
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
        visited_nodes.push(node.process_id);
        node.blockers = self.find_blockers(state, node.process_id);

        for mut blocker in &mut node.blockers {
            if blocker.process_id == root_node_id {
                return vec!(blocker.clone()); // add the last node in the loop to end of trail
            }

            // if we've visited this blocking node before, then we've detected a loop
            if !visited_nodes.contains(&blocker.process_id) {
                let mut blocker_subtree = self.traverse_blocker_tree(state, visited_nodes,
                                                                     root_node_id, blocker);
                if blocker_subtree.len() > 0 {
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
        display_string.push_str(&format!("#{}", root_node.process_id));
        for node in node_set {
            display_string.push_str(&format!("{}", node));
        }
        display_string
    }

    fn deadlock_inspection(&self, state: &RunState) {
        for blocked_process_id in state.get_blocked() {
            // start a clean tree with a new root node for each blocked process
            let mut root_node = BlockerNode::new(*blocked_process_id, 0, BlockType::OutputBlocked);
            let mut visited_nodes = vec!();

            let mut deadlock_set = self.traverse_blocker_tree(state, &mut visited_nodes,
                                                              *blocked_process_id, &mut root_node);
            if deadlock_set.len() > 0 {
                self.client.display(&format!("Deadlock detected\n"));
                self.client.display(&format!("{}\n", Self::display_set(&root_node, deadlock_set)));
            }
        }
    }

    fn print_function(&self, state: &RunState, function_id: usize) {
        let function = state.get(function_id);
        self.client.display(&format!("{}", function));
        self.client.display(&state.display_state(function_id));
    }

    fn print_all_processes(&self, state: &RunState) {
        for id in 0..state.num_functions() {
            self.print_function(state, id);
        }
    }

    fn print(&self, state: &RunState, param: Option<Param>) {
        match param {
            None => self.client.display(&format!("{}\n", state)),
            Some(Param::Numeric(function_id)) |
            Some(Param::Block((function_id, _))) => self.print_function(state, function_id),
            Some(Param::Input((function_id, _))) => self.print_function(state, function_id),
            Some(Param::Wildcard) => self.print_all_processes(state),
            Some(Param::Output(_)) => self.client.display(
                "Cannot display the output of a process until it is executed. \
                Set a breakpoint on the process by id and then step over it")
        }
    }

    fn step(&mut self, state: &RunState, steps: Option<Param>) {
        match steps {
            None => self.break_at_job = state.jobs() + 1,
            Some(Param::Numeric(steps)) => self.break_at_job = state.jobs() + steps,
            _ => self.client.display("Did not understand step command parameter\n")
        }
    }

    fn parse_command(input: &String) -> (&str, Option<Param>) {
        let parts: Vec<&str> = input.trim().split(' ').collect();
        let command = parts[0];

        if parts.len() > 1 {
            if parts[1] == "*" {
                return (command, Some(Param::Wildcard));
            }

            match parts[1].parse::<usize>() {
                Ok(integer) => return (command, Some(Param::Numeric(integer))),
                Err(_) => { /* not an integer - fall through */ }
            }

            if parts[1].contains("/") { // is an output specified
                let sub_parts: Vec<&str> = parts[1].split('/').collect();
                match sub_parts[0].parse::<usize>() {
                    Ok(source_process_id) =>
                        return (command, Some(Param::Output((source_process_id, sub_parts[1].to_string())))),
                    Err(_) => { /* couldn't parse source process id */ }
                }
            } else if parts[1].contains(":") { // is an input specifier
                let sub_parts: Vec<&str> = parts[1].split(':').collect();
                match (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
                    (Ok(dest_process_id), Ok(dest_input_number)) =>
                        return (command, Some(Param::Input((dest_process_id, dest_input_number)))),
                    (_, _) => { /* couldn't parse the process and input numbers */ }
                }
            } else if parts[1].contains("->") { // is a block specifier
                let sub_parts: Vec<&str> = parts[1].split("->").collect();
                match (sub_parts[0].parse::<usize>(), sub_parts[1].parse::<usize>()) {
                    (Ok(blocked_process_id), Ok(blocking_process_id)) =>
                        return (command, Some(Param::Block((blocked_process_id, blocking_process_id)))),
                    (_, _) => { /* couldn't parse the process ids */ }
                }
            }
        }

        (command, None)
    }
}
