use description::flow::Flow;

/*
 dump a valid flow to stdout
 */
pub fn dump(flow: Flow, level: usize) {
    // TODO Doesn't work :-(
    println!("{:indent$}{}", flow, indent=level * 4);

    // Dump sub-flows
    for subflow in flow.flows {
        dump(subflow, level +1);
    }
}