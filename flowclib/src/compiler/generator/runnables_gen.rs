use strfmt::Result;
use strfmt::strfmt;
use std::collections::HashMap;

const RUNNABLES_TEMPLATE: &'static str = "
use flowrlib::value::Value;
use flowrlib::runnable::Runnable;
use flowrlib::function::Function;
use flowstdlib::stdio::stdout::Stdout;
use std::sync::{{Arc, Mutex}};

pub fn get_runnables() -> Vec<Arc<Mutex<Runnable>>> {{
    let mut runnables = Vec::<Arc<Mutex<Runnable>>>::with_capacity(2);

    runnables.push(Arc::new(Mutex::new(Value::new(Some(\"Hello-World\"),
                                       vec!((1,0))))));
    runnables.push(Arc::new(Mutex::new(Function::new(&Stdout{{}},
                                          vec!()))));
    runnables
}}
";

pub fn runnables_file_contents(vars: &HashMap<String, &str>) -> Result<String>{
    strfmt(RUNNABLES_TEMPLATE, &vars)
}
