/*#![no_main]
#![no_std]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}*/

use flow_macro::flow_function;
use serde_json::json;
use serde_json::Value;

#[flow_function]
fn _reverse(inputs: &[Value]) -> (Option<Value>, bool) {
    let mut value = None;

    if inputs.len() == 1 {
        let input = &inputs[0];
        if let Value::String(ref s) = input {
            value = Some(json!({
                "reversed" : s.chars().rev().collect::<String>(),
                "original": s
            }));
        }
    }

    (value, RUN_AGAIN)
}

#[cfg(test)]
mod test {
    use flowcore::{RUN_AGAIN};
    use serde_json::json;
    use super::_reverse;

    #[test]
    fn invalid_input() {
        let (value, run_again) = _reverse(&[]);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_string() {
        let string = "string of text";
        let value = json!(string);
        let (value, run_again) = _reverse(&[value]);

        assert_eq!(run_again, RUN_AGAIN);
        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("reversed").expect("Could not get string args"),
            &json!("txet fo gnirts")
        );
    }
}
