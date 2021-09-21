/*#![no_main]
#![no_std]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}*/

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN};
use serde_json::json;
use serde_json::Value;

#[derive(FlowImpl, Debug)]
pub struct Reverser;

impl Implementation for Reverser {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, bool) {
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
}

#[cfg(test)]
mod test {
    use flowcore::{Implementation, RUN_AGAIN};
    use serde_json::json;

    use super::Reverser;

    #[test]
    fn invalid_input() {
        let reverser = &Reverser {} as &dyn Implementation;
        let (value, run_again) = reverser.run(&[]);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_string() {
        let string = "string of text";
        let value = json!(string);
        let reverser = &Reverser {} as &dyn Implementation;
        let (value, run_again) = reverser.run(&[value]);

        assert_eq!(run_again, RUN_AGAIN);
        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("reversed").expect("Could not get string args"),
            &json!("txet fo gnirts")
        );
    }
}
