use runnable::Runnable;
use value::Value;
use function::Function;
use std::process::exit;

/*
    This function is responsible for loading triggering the availability of the inputs to Values
    that have initial_value specified in their definition. That will update their "runnable"
    status and trigger making the value available on it's output to all connected inputs.
*/
fn init(values: &mut Vec<Box<Value>>) {
    for ref mut value in values {
        value.init();
    }
}

pub fn looper(mut values: Vec<Box<Value>>, functions: Vec<Box<Function>>) -> ! {
    let mut blocked = Vec::<Box<Runnable>>::new();
    let mut ready = Vec::<Box<Runnable>>::new();

    println!("Initializing values");
    for mut value in values {
        if value.init() {
            ready.push(value);
        } else {
            blocked.push(value);
        }
    }

    for function in functions {
        blocked.push(function);
    }

    println!("Starting execution loop");
    loop {
        for mut runnable in ready {
            runnable.run();
        }

        // for everything that is listening on the output of the function/value that was just
        // run... (if the function produces no output, then no one will be listening and null list
        // their status needs to be checked...
//        functions[0].implementation.run(&mut functions[0]);

        exit(-1);
    }
}