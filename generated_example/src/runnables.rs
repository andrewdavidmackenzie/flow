use flowrlib::value::Value;
use flowrlib::runnable::Runnable;
use flowrlib::function::Function;
use flowstdlib::stdio::stdout::Stdout;
use std::sync::{Arc, Mutex};

pub fn get_runnables() -> Vec<Arc<Mutex<Runnable>>> {
    let mut runnables = Vec::<Arc<Mutex<Runnable>>>::with_capacity(2);

    runnables.push(Arc::new(Mutex::new(Value::new(0, Some("Hello-World"),
                                       vec!((1,0))))));
    runnables.push(Arc::new(Mutex::new(Function::new(1, &Stdout{},
                                          vec!()))));
    runnables
}