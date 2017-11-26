use description::flow::Flow;

/*
    dump a valid flow to stdout
 */
pub fn dump(flow: &Flow, level: usize) {
    // TODO Doesn't work :-(
    println!("{:indent$}{}", "", flow, indent=(level * 4));

    // Dump sub-flows
    if let Some(ref flow_refs) = flow.flow {
        for flow_ref in flow_refs {
            dump(&flow_ref.flow, level +1);
        }
    }
}