use model::flow::Flow;


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
/// url = url.join("samples/hello-world-simple/context.toml").unwrap();
/// let flow = flowclib::loader::loader::load(&url).unwrap();
/// flowclib::dumper::dumper::dump(&flow)
/// ```
///
pub fn dump(flow: &Flow) {
    _dump(flow, 0);
}

/*
    dump the flow definition recursively, tracking what leverl we are at as we go down
*/
fn _dump(flow: &Flow, level: usize) {
    println!("\nLevel={}\n{}", level, flow);

    // Dump sub-flows
    if let Some(ref flow_refs) = flow.flow_refs {
        for flow_ref in flow_refs {
            _dump(&flow_ref.flow, level +1);
        }
    }
}