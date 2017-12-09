use value::Value;
use function::Function;
use std::process::exit;

/*
    This function is responsible for loading triggering the availability of the inputs to Values
    that have initial_value specified in their definition. That will update their "runnable"
    status and trigger making the value available on it's output to all connected inputs.
*/
fn init(values: Vec<Value> ) {
    println!("Initializing values");
    for ref mut value in values {
        value.init();
    }
}

pub fn looper(values: Vec<Value> , mut functions: Vec<Function>) -> ! {
    println!("Starting execution loop");

    // TODO at the start assume all functions are blocked

    // init may produce outputs from values - unblocking something
    init(values);

    loop {
        // for each function in runnable list
        // call it's run function - that will make it's output (if it has any) available for others

        // for everything that is listening on the output of the function/value that was just
        // run... (if the function produces no output, then no one will be listening and null list
        // their status needs to be checked...
        functions[0].implementation.run(&mut functions[0]);

        exit(-1);
    }
}