use serde_json::Value as JsonValue;

/*
    Given the row and column of a pixel in the output image, return the
    corresponding point on the complex plane.

    `bounds` is a pair giving the width and height of the image in pixels.
    `pixel` is a (row, column) pair indicating a particular pixel in that image.
    The `upper_left` and `lower_right` parameters are points on the complex
    plane designating the area our image covers.
*/
#[no_mangle]
pub extern "C" fn create_complex(mut inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, bool) {
    let mut value = None;

    let arg1 = inputs.remove(0).remove(0);
    let arg2 = inputs.remove(0).remove(0);

    match (arg1, arg2) {
        (JsonValue::Number(re), JsonValue::Number(im)) => {
            value = Some(json!({ "re" : re, "im": im }));
        }
        _ => {}
    }

    (value, true)
}

#[cfg(test)]
mod tests {
    use serde_json::Value as JsonValue;

    #[test]
    fn parse_complex_ok() {
        // Create input args - two floating point numbers
        let arg1 = json!(1.5);
        let arg2 = json!(1.6);

        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(arg1), vec!(arg2));

        let _complex = super::create_complex(inputs);
    }
}


