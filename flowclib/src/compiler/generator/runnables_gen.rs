use strfmt::Result;
use strfmt::strfmt;
use std::collections::HashMap;
use flowrlib::runnable::Runnable;

const RUNNABLES_PREFIX: &'static str = "
use flowrlib::value::Value;
use flowrlib::runnable::Runnable;
use flowrlib::function::Function;
use flowstdlib::stdio::stdout::Stdout;
use std::sync::{{Arc, Mutex}};

pub fn get_runnables() -> Vec<Arc<Mutex<Runnable>>> {{
    let mut runnables = Vec::<Arc<Mutex<Runnable>>>::with_capacity(2);\n\n";

const RUNNABLES_SUFFIX: &'static str = "
    runnables
}}";

pub fn runnables_file_contents(vars: &HashMap<String, &str>,
                               runnables: Vec<Box<Runnable>>) -> Result<String> {
    let mut content = strfmt(RUNNABLES_PREFIX, &vars)?;

    for runnable in runnables {
        let run_str = format!("    runnables.push(Arc::new(Mutex::new({})));\n", runnable.to_code());
        content.push_str(&run_str);
    }

    let suffix = strfmt(RUNNABLES_SUFFIX, &vars)?;
    content.push_str(&suffix);
    Ok(content)
}
