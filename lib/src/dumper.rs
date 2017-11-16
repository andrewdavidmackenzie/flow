
use description::context::Context;

/*
 dump a valid context to stdout
 */
pub fn dump(context: Context) {
    println!("Context: \n{}", context);

    match context.flow {
        Some(flow) => {
            println!("Flow: \n{}", flow);
        },
        None => {}
    }
}