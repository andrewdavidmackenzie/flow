use model::flow::Flow;
use std::fmt;
use generator::code_gen::CodeGenTables;

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

pub fn dump_tables(tables: &CodeGenTables) {
    print(tables.connections.iter(), "Collapsed Connections");
    print(tables.runnables.iter(), "Runnables");
    print(tables.libs.iter(), "Libraries");
    print(tables.lib_references.iter(), "Library references");
}

fn print<C: Iterator>(table: C, title: &str) where <C as Iterator>::Item: fmt::Display {
    println!("\n{}:", title);
    for e in table.into_iter() {
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