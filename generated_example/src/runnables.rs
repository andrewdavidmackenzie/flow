use flowrlib::value::Value;
use flowrlib::runnable::Runnable;
use flowrlib::function::Function;
use flowstdlib::stdio::stdout::Stdout;

pub fn get_runnables() -> Vec<Box<Runnable>> {
    let mut runnables = Vec::<Box<Runnable>>::with_capacity(2);

    runnables.push(Box::new(Value::new(Some("Hello-World"), 1)));
    runnables.push(Box::new(Function::new(&Stdout{}, 0)));

    runnables
}