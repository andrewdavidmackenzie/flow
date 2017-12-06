use value::Value;
use function::Function;
use std::process::exit;

/*
    This function is responsible for loading triggering the availability of the inputs to Values
    that have initial_value specified in their definition. That will update their "runnable"
    status and trigger making the value available on it's output to all connected inputs.
*/
pub fn init(values: &[&'static Value]) {
    println!("Loading values");
    for &value in values.iter() {
        if let Some(initial_value) = value.initial_value {
            println!("initial_value: {}", initial_value);
        }
    }
}

pub fn looper(_values: &[&'static Value], _functions: &[&'static (Function+Sync)]) -> ! {
    println!("Starting execution loop with values and functions");
    loop {
        println!("Nothing to do yet...");
        exit(-1);
    }
}