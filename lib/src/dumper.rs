use model::flow::Flow;

/*
    dump a valid flow to stdout
 */
pub fn dump(flow: &Flow, level: usize) {
    println!("\nLevel={}\n{}", level, flow);

    // Dump sub-flows
    if let Some(ref flow_refs) = flow.flow {
        for flow_ref in flow_refs {
            dump(&flow_ref.flow, level +1);
        }
    }
}