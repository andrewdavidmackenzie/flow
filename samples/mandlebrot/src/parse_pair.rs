use serde_json::Value as JsonValue;
use std::str::FromStr;

pub struct ParsePair;

/*
    Given the row and column of a pixel in the output image, return the
    corresponding point on the complex plane.

    `bounds` is a pair giving the width and height of the image in pixels.
    `pixel` is a (row, column) pair indicating a particular pixel in that image.
    The `upper_left` and `lower_right` parameters are points on the complex
    plane designating the area our image covers.
*/
fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList)
    -> (JsonValue, bool) {
    let string = inputs.remove(0).remove(0);
    let separator = inputs.remove(0).remove(0);

    match (string, separator) {
        (JsonValue::String(string_value), JsonValue::String(seperator_value)) => {
            let split: Option<(f64, f64)> = parse_pair(string_value.as_str(),
                                                       seperator_value.as_str());

            // send output as Json
            if let Some(pair) = split {
                let output = json!({ "first" : pair.0, "second": pair.1 });
                run_list.send_output(process, output);
            }
        }
        _ => {}
    }

    true
}

/// Parse the string 's' as a coordinate pair, like "400x600" or "1.0,0.5"
/// Specifically, 's' should have the form <left><sep><right> where <sep> is the character given by
/// the 'separator' argument, and <left> and <right> are both strings that can be parsed
/// by 'T::from_str'.
/// If 's' has the proper form, return 'Some<(x,y)>'.
/// If 's' doesn't parse correctly, return None.
pub fn parse_pair<T: FromStr>(s: &str, separator: &str) -> Option<(T, T)> {
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
    use flowrlib::process::Process;
    use flowrlib::runlist::RunList;
    use serde_json::Value as JsonValue;

    use super::parse_pair;
    use super::ParsePair;

    #[test]
    fn parse_pair_bounds() {
        // Create argument a set of bounds separated by an 'x'
        let string = json!("100x200");
        let separator = json!("x");

        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(string), vec!(separator));

        let mut run_list = RunList::new();
        let pp = &Process::new("pp", 2, true, vec!(1, 1, 1),
                               0,
                               Box::new(ParsePair),
                               None,
                               vec!()) as &Process;
        let implementation = pp.implementation();

        implementation.run(pp, inputs, &mut run_list);
    }

    #[test]
    fn test_parse_pair() {
        assert_eq!(parse_pair::<i32>("", ","), None);
        assert_eq!(parse_pair::<i32>("10,", ","), None);
        assert_eq!(parse_pair::<i32>(",10", ","), None);
        assert_eq!(parse_pair::<i32>("10,20", ","), Some((10, 20)));
        assert_eq!(parse_pair::<i32>("10,20xy", ","), None);
        assert_eq!(parse_pair::<f64>("0.5x", ","), None);
        assert_eq!(parse_pair::<f64>("0.5x1.5", "x"), Some((0.5, 1.5)));
    }
}


