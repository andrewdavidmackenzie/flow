use serde_json::Value;
use crate::run_state::Output;

pub enum Param {
    Wildcard,
    Numeric(usize),
    Output((usize, String)),
    Input((usize, usize)),
    Block((usize, usize)),
}

pub enum Command {
    Breakpoint(Option<Param>),
    Continue,
    Delete(Option<Param>),
    ExitDebugger,
    Inspect,
    List,
    Print(Option<Param>),
    RunReset,
    Step(Option<Param>),
    GetState,
    GetFunctionState(usize)
}

pub enum Event {
    JobCompleted(usize, usize, Option<Value>), // job_id, function_id
    Start,
    PriorToSendingJob(usize, usize), // job_id, function_id
    BlockBreakpoint(usize, usize, usize), // blocked_id, blocking_id, blocking_io_number
    DataBreakpoint(usize, String, Value, usize, usize), // source_process_id, output_route, value, destination_id, input_number));
    Panic(Output), // output of job that panicked
    RuntimeError(String), // message resulting from the error
    End,
    Deadlock(String),
    SendingValue(usize, Value, usize, usize), // source_process_id, value, destination_id, input_number
}

pub enum Response {
    Ack,
    Error(String),
    Message(String),
    Resetting,
    Running,
    Exiting
}

pub trait DebugClient {
    fn init(&self);
    fn get_command(&self, job_number: Option<usize>) -> Command;
    fn send_event(&self, event: Event);
    fn send_response(&self, response: Response);
}