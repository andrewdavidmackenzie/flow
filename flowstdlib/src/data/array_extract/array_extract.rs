use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;
use serde_json::{json, Value};

fn resolve_index(index: i64, len: usize) -> usize {
    if index < 0 {
        let abs = usize::try_from(index.unsigned_abs()).unwrap_or(len);
        len.saturating_sub(abs)
    } else {
        usize::try_from(index).unwrap_or(len).min(len)
    }
}

#[flow_function]
fn inner_array_extract(
    array: &Value,
    start: &Value,
    end: &Value,
) -> Result<(Option<Value>, RunAgain)> {
    let array = array.as_array().ok_or("Could not get array")?;
    let start_idx = start.as_i64().ok_or("Could not get start as i64")?;
    let end_idx = end.as_i64().ok_or("Could not get end as i64")?;

    let len = array.len();
    let s = resolve_index(start_idx, len);
    let e = resolve_index(end_idx, len);

    let slice = if s < e {
        array.get(s..e).map_or_else(Vec::new, <[Value]>::to_vec)
    } else {
        vec![]
    };

    flow_output!("slice" => json!(slice))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_array_extract;

    #[test]
    fn extract_middle() {
        let (result, _) = inner_array_extract(&json!([10, 20, 30, 40, 50]), &json!(1), &json!(4))
            .expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/slice").expect("no /slice"),
            json!([20, 30, 40])
        );
    }

    #[test]
    fn extract_with_negative_start() {
        let (result, _) = inner_array_extract(&json!([10, 20, 30, 40, 50]), &json!(-3), &json!(5))
            .expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/slice").expect("no /slice"),
            json!([30, 40, 50])
        );
    }

    #[test]
    fn extract_with_negative_end() {
        let (result, _) = inner_array_extract(&json!([10, 20, 30, 40, 50]), &json!(0), &json!(-1))
            .expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/slice").expect("no /slice"),
            json!([10, 20, 30, 40])
        );
    }

    #[test]
    fn drop_first() {
        let (result, _) =
            inner_array_extract(&json!([10, 20, 30]), &json!(1), &json!(3)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/slice").expect("no /slice"),
            json!([20, 30])
        );
    }

    #[test]
    fn drop_last() {
        let (result, _) =
            inner_array_extract(&json!([10, 20, 30]), &json!(0), &json!(-1)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/slice").expect("no /slice"),
            json!([10, 20])
        );
    }

    #[test]
    fn last_n_elements() {
        let (result, _) = inner_array_extract(&json!([10, 20, 30, 40, 50]), &json!(-2), &json!(5))
            .expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/slice").expect("no /slice"),
            json!([40, 50])
        );
    }

    #[test]
    fn empty_when_start_past_end() {
        let (result, _) =
            inner_array_extract(&json!([10, 20, 30]), &json!(3), &json!(1)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(*output.pointer("/slice").expect("no /slice"), json!([]));
    }

    #[test]
    fn empty_array_input() {
        let (result, _) = inner_array_extract(&json!([]), &json!(0), &json!(5)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(*output.pointer("/slice").expect("no /slice"), json!([]));
    }

    #[test]
    fn clamps_to_bounds() {
        let (result, _) =
            inner_array_extract(&json!([10, 20, 30]), &json!(-10), &json!(100)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/slice").expect("no /slice"),
            json!([10, 20, 30])
        );
    }
}
