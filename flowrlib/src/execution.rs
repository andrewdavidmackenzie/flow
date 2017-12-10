use runnable::Runnable;
use value::Value;
use function::Function;
use std::process::exit;

/*
    This function is responsible for initializing the value of any Values that have initial_value
    specified in their definition, and if they do, placing them in the `ready` queue.

    It places all unitialized values and all functions in the `blocked` queue pending inputs.
*/
fn init(values: Vec<Box<Value>>, functions: Vec<Box<Function>>,
        blocked: &mut Vec<Box<Runnable>>, ready: &mut Vec<Box<Runnable>>) {
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
}

/// The generated code for a flow consists of values and functions. Once these lists have been
/// loaded at program start-up then start executing the program using the `execute` method.
/// You should not have to write code to use this method yourself, it will be called from the
/// generated code in the `main` method.
///
/// It is a divergent function that will never return. On completion of the execution of the flow
/// it will exit the process.
///
/// # Example
/// ```
/// # use flowrlib::function::Function;
/// # use flowrlib::value::Value;
/// use flowrlib::execution::execute;
///
/// let values = Vec::<Box<Value>>::new();
/// let functions = Vec::<Box<Function>>::new();
///
/// execute(values, functions);
/// ```
pub fn execute(values: Vec<Box<Value>>, functions: Vec<Box<Function>>) -> ! {
    let mut blocked = Vec::<Box<Runnable>>::new();
    let mut ready = Vec::<Box<Runnable>>::new();

    init(values, functions, &mut blocked, &mut ready);

    println!("Starting execution loop");
    loop {
        for mut runnable in ready {
            runnable.run();
        }

        // for everything that is listening on the output of the function/value that was just
        // run... (if the function produces no output, then no one will be listening and null list
        // their status needs to be checked...
//        functions[0].implementation.run(&mut functions[0]);

        exit(0);
    }
}