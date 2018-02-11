use strfmt::Result;
use strfmt::strfmt;
use std::collections::HashMap;
use flowrlib::runnable::Runnable;

const RUNNABLES_PREFIX: &'static str = "
// Flow Run-time library references
use flowrlib::value::Value;
use flowrlib::runnable::Runnable;
use flowrlib::function::Function;

// Rust std library references
use std::sync::{{Arc, Mutex}};\n";

const GET_RUNNABLES: &'static str = "
pub fn get_runnables() -> Vec<Arc<Mutex<Runnable>>> {{
    let mut runnables = Vec::<Arc<Mutex<Runnable>>>::with_capacity(2);\n\n";

const RUNNABLES_SUFFIX: &'static str = "
    runnables
}}";

pub fn contents(vars: &HashMap<String, &str>,
                lib_references: Vec<&str>,
                runnables: Vec<Box<Runnable>>) -> Result<String> {
    let mut content = strfmt(RUNNABLES_PREFIX, &vars)?;

    content.push_str("\n// Library functions\n");
    for lib_ref in lib_references {
        content.push_str(lib_ref);
    }

    let get_runnables = strfmt(GET_RUNNABLES, &vars)?;
    content.push_str(&get_runnables);

    for runnable in runnables {
        let run_str = format!("    runnables.push(Arc::new(Mutex::new({})));\n", runnable.to_code());
        content.push_str(&run_str);
    }

    let suffix = strfmt(RUNNABLES_SUFFIX, &vars)?;
    content.push_str(&suffix);
    Ok(content)
}
