use serde_json::json;
use serde_json::Value;
use std::str::FromStr;

/*
    Given the row and column of a pixel in the output image, return the
    corresponding point on the complex plane.

    `bounds` is a pair giving the width and height of the image in pixels.
    `pixel` is a (row, column) pair indicating a particular pixel in that image.
    The `upper_left` and `lower_right` parameters are points on the complex
    plane designating the area our image covers.
*/
#[no_mangle]
pub extern "C" fn parse_pair(mut inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
    let mut value = None;

    let string = inputs.remove(0).remove(0);
    let separator = inputs.remove(0).remove(0);

    match (string, separator) {
        (Value::String(string_value), Value::String(seperator_value)) => {
            let split: Option<(f64, f64)> = _parse_pair(string_value.as_str(),
                                                        seperator_value.as_str());

            // send output as Json
            if let Some(pair) = split {
                value = Some(json!({ "first" : pair.0, "second": pair.1 }));
            }
        }
        _ => {}
    }

    (value, true)
}

fn _parse_pair<T: FromStr>(s: &str, separator: &str) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
                (Ok(l), Ok(r)) => Some((l, r)),
                _ => None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::_parse_pair;

    #[test]
    fn parse_pair_bounds() {
        // Create argument a set of bounds separated by an 'x'
        let string = json!("100x200");
        let separator = json!("x");

        let inputs: Vec<Vec<Value>> = vec!(vec!(string), vec!(separator));

        let _pair = super::parse_pair(inputs);
    }
}


