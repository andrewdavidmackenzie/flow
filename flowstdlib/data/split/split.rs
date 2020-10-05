use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::{json, Value};

#[derive(FlowImpl)]
/// Split a string into (possibly) two parts and a possible token, based on a separator.
///
/// This function is implemented in a deliberate way to be able to showcase parallelization.
///
/// Instead of going through the string in order looking for the separator and gathering an array
/// of sections it takes an alternative approach.
///
/// It starts in the middle of the string looking for a separator character from there towards the
/// end. If it finds one then the string is split in two and those two sub-strings are output as
/// an array of strings on the `partial` output. NOTE that either or both of these two sub-strings
/// may have separators within them, and hence need further sub-division.
///
/// For that reason, the `partial` output is feedback to the `string` input, and the runtime will
/// serialize the aarray of strings to the input as separate strings.
///
/// If from the middle to the end no separator is found, then it tries from the middle backwards
/// towards the beginning. If a separator is found, the two sub-strings are output on `partial`
/// output as before.
///
/// If no separator is found in either of those cases, then the string doesn't have any and is
/// output on the `token` output.
///
/// Thus, strings with separators are sub-divided until strings without separators are found, and
/// each of those is output as a token.
///
/// Due to the splitting and recursion approach, the order of the output tokens is not the order
/// they appear in the string.
#[derive(Debug)]
pub struct Split;

impl Implementation for Split {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if inputs[0].is_string() {
            let string = inputs[0].as_str().unwrap();
            let separator = inputs[1].as_str().unwrap();

            let (partial, token) = split(string, separator);

            let mut output_map = serde_json::Map::new();

            let mut work_delta: i32 = -1; // we have consumed a string, so one down

            if let Some(partial) = partial {
                // but we have generated some new strings to be processed by other jobs
                work_delta += partial.len() as i32;
                output_map.insert("partial".into(), json!(partial));
            }

            output_map.insert("delta".into(), json!(work_delta));

            if let Some(tok) = token {
                output_map.insert("token".into(), json!(tok));
                output_map.insert("token-count".into(), json!(1));
            } else {
                output_map.insert("token-count".into(), json!(0));
            }

            let output = Value::Object(output_map);

            (Some(output), RUN_AGAIN)
        } else {
            (None, RUN_AGAIN)
        }
    }
}

// Separate an array of text at a separator string close to the center, dividing in two if possible
fn split(input: &str, separator: &str) -> (Option<Vec<String>>, Option<String>) {
    let text = input.trim();

    if text.is_empty() {
        return (None, None);
    }

    if text.len() < 3 {
        return (None, Some(text.to_string()));
    }

    let start = 0;
    let middle = text.len() / 2;
    let end = text.len();

    // try and find a separator from middle towards the end
    for point in middle..end { // cannot have separator at end
        if text.get(point..point + 1).unwrap() == separator {
            return (Some(vec!(text[0..point].to_string(), text[point + 1..text.len()].to_string())), None);
        }
    }

    // try and find a separator from middle backwards towards the start
    for point in (start..middle).rev() {
        if text.get(point..point + 1).unwrap() == separator {
            // If we find one return the string upto that  point for further splitting, plus the string from
            // there to the end as a token
            return (Some(vec!(text[0..point].to_string())), Some(text[point + 1..text.len()].to_string()));
        }
    }

    // No separator found - return entire string as one entry in the vector
    (None, Some(text.to_string()))
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::json;

    #[test]
    fn basic_tests() {
        #[allow(clippy::type_complexity)]
            let test_cases: Vec<(&str, (Option<Vec<String>>, Option<String>))> = vec!(
            // empty string case
            ("", (None, None)),

            // just separators
            (" ", (None, None)),   // 1
            ("  ", (None, None)),  // 2
            ("   ", (None, None)), // 3

            // one letter words
            ("a", (None, Some("a".into()))),   // no separator
            (" a", (None, Some("a".into()))),  // separator before
            ("a ", (None, Some("a".into()))),  // separator after
            (" a ", (None, Some("a".into()))), // separator before and after

            // two letter words
            ("aa", (None, Some("aa".into()))),   // no separator
            (" aa", (None, Some("aa".into()))),  // separator before
            ("aa ", (None, Some("aa".into()))),  // separator after
            (" aa ", (None, Some("aa".into()))), // separator before and after

            // One word texts
            ("text", (None, Some("text".into()))),   // no separator
            (" text", (None, Some("text".into()))),  // separator before
            ("text ", (None, Some("text".into()))),  // separator after
            (" text ", (None, Some("text".into()))), // separator before and after

            // Two word texts
            ("some text", (Some(vec!("some".into(), "text".into())), None)),   // separator in middle
            ("some text ", (Some(vec!("some".into(), "text".into())), None)),  // separator in middle and after
            (" some text", (Some(vec!("some".into(), "text".into())), None)),  // separator before, middle
            (" some text ", (Some(vec!("some".into(), "text".into())), None)), // separator before, middle and after

            // longer texts
            ("the quick brown fox jumped over the lazy dog",
             (Some(vec!("the quick brown fox jumped".into(), "over the lazy dog".into())), None)),
            ("the quick brown fox jumped-over-the-lazy-dog",
             (Some(vec!("the quick brown fox".into())), Some("jumped-over-the-lazy-dog".into()))),
            ("the-quick-brown-fox-jumped-over-the-lazy-dog",
             (None, Some("the-quick-brown-fox-jumped-over-the-lazy-dog".into()))),
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
        let separator = json!(" ");
        let mut output_vector = vec!();
        let mut input_strings = vec!(string);

        loop {
            // if nothing else to split we're dong
            if input_strings.is_empty() {
                break;
            }

            let this_input = input_strings.pop().unwrap();
            let splitter = super::Split {};
            let (result, _) = splitter.run(&[this_input, separator.clone()]);

            let output = result.unwrap();
            if let Some(token) = output.pointer("/token") {
                output_vector.push(token.clone());
            }

            if let Some(split_values) = output.pointer("/partial") {
                for value in split_values.as_array().unwrap().iter() {
                    input_strings.push(value.clone());
                }
            }
        }
    }

    #[test]
    fn no_partials_no_token_work_delta() {
        let test = (json!("  "), 1);
        let separator = json!(" ");

        let splitter = super::Split {};
        let (result, _) = splitter.run(&[test.0, separator]);

        let output = result.unwrap();
        assert!(output.pointer("/token").is_none());
        assert_eq!(output.pointer("/token-count").unwrap(), &json!(0));
        assert!(output.pointer("/partial").is_none());
        assert_eq!(output.pointer("/delta").unwrap(), &json!(-1));
    }

    #[test]
    fn two_partials_no_token_work_delta() {
        let test = (json!("the quick brown fox jumped over the lazy dog"), 1);
        let separator = json!(" ");

        let splitter = super::Split {};
        let (result, _) = splitter.run(&[test.0, separator]);

        let output = result.unwrap();
        assert!(output.pointer("/token").is_none());
        assert_eq!(output.pointer("/token-count").unwrap(), &json!(0));
        assert_eq!(output.pointer("/partial").unwrap(), &json!(["the quick brown fox jumped", "over the lazy dog"]));
        assert_eq!(output.pointer("/delta").unwrap(), &json!(1));
    }

    #[test]
    fn one_partial_one_token_work_delta() {
        let test = (json!("the quick brown fox-jumped-over-the-lazy-dog"), 1);
        let separator = json!(" ");

        let splitter = super::Split {};
        let (result, _) = splitter.run(&[test.0, separator]);

        let output = result.unwrap();
        assert_eq!(output.pointer("/token").unwrap(), "fox-jumped-over-the-lazy-dog");
        assert_eq!(output.pointer("/token-count").unwrap(), &json!(1));
        assert_eq!(output.pointer("/partial").unwrap(), &json!(["the quick brown"]));
        assert_eq!(output.pointer("/delta").unwrap(), &json!(0));
    }

    #[test]
    fn no_partials_one_token_work_delta() {
        let test = (json!("the"), -1);
        let separator = json!(" ");

        let splitter = super::Split {};
        let (result, _) = splitter.run(&[test.0, separator]);

        let output = result.unwrap();
        assert!(output.pointer("/partial").is_none());
        assert_eq!(output.pointer("/token").unwrap(), &json!("the"));
        assert_eq!(output.pointer("/token-count").unwrap(), &json!(1));
        assert_eq!(output.pointer("/delta").unwrap(), &json!(-1));
    }
}