use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::{json, Value};

#[derive(FlowImpl)]
/// Split a string into (possibly) two parts and a possible token, based on a separator
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "split"
/// source = "lib://flowstdlib/data/split"
/// ```
///
/// ## Inputs
/// * string - the String to split
///
/// * separator - the String to use as a separator
///
/// ## Outputs
/// * partial - an Array of Strings that each may or may not have `separator` strings inside
/// them. This should be feed-back to the input (will be serialized into Strings by the
/// runtime) for further subdivision until each one cannot be split further - in which case
/// it will be output as `token`
///
/// * token - a String that cannot be sub-divided further.
#[derive(Debug)]
pub struct Split;

impl Implementation for Split {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let string = inputs[0][0].as_str().unwrap();
        let separator = inputs[1][0].as_str().unwrap();

        let (partial, token) = split(string, separator);

        let mut output_map = serde_json::Map::new();
        output_map.insert("partial".into(), json!(partial));
        if let Some(tok) = token {
            output_map.insert("token".into(), json!(tok));
        }

        let output = Value::Object(output_map);

        (Some(output), RUN_AGAIN)
    }
}

// Separate an array of text at a separator string close to the center, dividing in two if possible
fn split(input: &str, separator: &str) -> (Vec<String>, Option<String>) {
    let text = input.trim();

    if text.is_empty() {
        return (Vec::new(), None);
    }

    if text.len() < 3 {
        return (Vec::new(), Some(text.to_string()));
    }

    let start = 0;
    let middle = text.len() / 2;
    let end = text.len();

    // try and find a separator from middle towards the end
    for point in middle..end { // cannot have separator at end
        if text.get(point..point + 1).unwrap() == separator {
            return (vec!(text[0..point].to_string(), text[point + 1..text.len()].to_string()), None);
        }
    }

    // try and find a separator from middle backwards towards the start
    for point in (start..middle).rev() {
        if text.get(point..point + 1).unwrap() == separator {
            return (vec!(text[0..point].to_string()), Some(text[point + 1..text.len()].to_string()));
        }
    }

    // No separator found - return entire string as one entry in the vector
    (Vec::new(), Some(text.to_string()))
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::json;

    #[test]
    fn basic_tests() {
        #[allow(clippy::type_complexity)]
        let test_cases: Vec<(&str, (Vec<&str>, Option<String>))> = vec!(
            // empty string case
            ("", (vec!(), None)),

            // just separators
            (" ", (vec!(), None)),   // 1
            ("  ", (vec!(), None)),  // 2
            ("   ", (vec!(), None)), // 3

            // one letter words
            ("a", (vec!(), Some("a".into()))),   // no separator
            (" a", (vec!(), Some("a".into()))),  // separator before
            ("a ", (vec!(), Some("a".into()))),  // separator after
            (" a ", (vec!(), Some("a".into()))), // separator before and after

            // two letter words
            ("aa", (vec!(), Some("aa".into()))),   // no separator
            (" aa", (vec!(), Some("aa".into()))),  // separator before
            ("aa ", (vec!(), Some("aa".into()))),  // separator after
            (" aa ", (vec!(), Some("aa".into()))), // separator before and after

            // One word texts
            ("text", (vec!(), Some("text".into()))),   // no separator
            (" text", (vec!(), Some("text".into()))),  // separator before
            ("text ", (vec!(), Some("text".into()))),  // separator after
            (" text ", (vec!(), Some("text".into()))), // separator before and after

            // Two word texts
            ("some text", (vec!("some", "text"), None)),   // separator in middle
            ("some text ", (vec!("some", "text"), None)),  // separator in middle and after
            (" some text", (vec!("some", "text"), None)),  // separator before, middle
            (" some text ", (vec!("some", "text"), None)), // separator before, middle and after

            // longer texts
            ("the quick brown fox jumped over the lazy dog", (vec!("the quick brown fox jumped", "over the lazy dog"), None)),
            ("the quick brown fox jumped-over-the-lazy-dog", (vec!("the quick brown fox"), Some("jumped-over-the-lazy-dog".into()))),
            ("the-quick-brown-fox-jumped-over-the-lazy-dog", (vec!(), Some("the-quick-brown-fox-jumped-over-the-lazy-dog".into()))),
        );

        for test in test_cases {
            let result = super::split(test.0, " ");
            let partial = result.0;
            let token = result.1;

            let expected_partial = (test.1).0;
            let expected_token = (test.1).1;

            assert_eq!(partial, expected_partial);
            assert_eq!(token, expected_token);
        }
    }

    #[test]
    fn iterate_until_done() {
        let string = json!("the quick brown fox jumped over the lazy dog");
        let separator = vec!(json!(" "));
        let mut output_vector = vec!();
        let mut input_strings = vec!(string);

        loop {
            // if nothing else to split we're dong
            if input_strings.is_empty() {
                break;
            }

            let this_input = input_strings.pop().unwrap();
            let splitter = super::Split {};
            let (result, _) = splitter.run(&vec!(vec!(this_input), separator.clone()));

            let output = result.unwrap();
            if let Some(token) = output.pointer("/token") {
                output_vector.push(token.clone());
            }
            let split_values = output.pointer("/partial").unwrap().as_array().unwrap().iter();
            for value in split_values {
                input_strings.push(value.clone());
            }
        }

        println!("output vector = {:?}", output_vector);
    }
}