use model::flow::Flow;
use model::value::Value;
use model::function::Function;
use model::connection::Connection;
use flowrlib::runnable::Runnable;
use std::fmt;

/// dump a flow definition that has been loaded to stdout
///
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
/// use std::env;
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// println!("url = {:?}", url);
/// url = url.join("samples/hello-world-simple/context.toml").unwrap();
/// let flow = flowclib::loader::loader::load(&url).unwrap();
/// flowclib::dumper::dumper::dump_flow(&flow)
/// ```
///
pub fn dump_flow(flow: &Flow) {
    _dump(flow, 0);
}

pub fn dump_tables(connections: &Vec<Connection>, values: &Vec<Value>, functions: &Vec<Function>,
                   runnables: &Vec<Box<Runnable>>, libs: &Vec<String>, lib_references: &Vec<String>) {
    print(connections, "Collapsed Connections");
    print(values, "Values");
    print(functions, "Functions");
    print(runnables, "Runnables");
    print(libs, "Libraries");
    print(lib_references, "Library references");
}

fn print<E: fmt::Display>(table: &Vec<E>, title: &str) {
    println!("\n{}:", title);
    for e in table.iter() {
        println!("{}", e);
    }
}

/*
    dump the flow definition recursively, tracking what leverl we are at as we go down
*/
fn _dump(flow: &Flow, level: usize) {
    println!("\nLevel={}\n{}", level, flow);

    // Dump sub-flows
    if let Some(ref flow_refs) = flow.flow_refs {
        for flow_ref in flow_refs {
            _dump(&flow_ref.flow, level + 1);
        }
    }
}