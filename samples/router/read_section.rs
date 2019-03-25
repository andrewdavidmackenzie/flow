use serde_json::Value;

#[no_mangle]
pub extern "C" fn run(mut inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
    let mut value = None;

    let input_stream = inputs.remove(0);
    let ra = input_stream[0].as_str().unwrap().parse::<u64>();
    let rb = input_stream[1].as_str().unwrap().parse::<u64>();
    let rc = input_stream[2].as_str().unwrap().parse::<u64>();

    match (ra, rb, rc) {
        (Ok(a), Ok(b), Ok(c)) => {
            value = Some(json!([a, b, c]));
        }
        _ => {}
    }

    (value, true)
}