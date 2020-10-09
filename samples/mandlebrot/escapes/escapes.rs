use num::Complex;
use serde_json::Value;

/*
    Try to determine if 'c' is in the Mandlebrot set, using at most 'limit' iterations to decide
    If 'c' is not a member, return 'Some(i)', where 'i' is the number of iterations it took for 'c'
    to leave the circle of radius two centered on the origin.
    If 'c' seems to be a member (more precisely, if we reached the iteration limit without being
    able to prove that 'c' is not a member) return 'None'
*/
#[no_mangle]
pub extern "C" fn escapes(mut inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
    let point = inputs.remove(0).remove(0);
    // pixel_bounds: (usize, usize),
    let re = point["re"].as_f64().unwrap();
    let im = point["im"].as_f64().unwrap();
    let complex_point = Complex { re, im };

    let limit = inputs.remove(0).remove(0).as_u64().unwrap();

    let value = Some(json!(_escapes(complex_point, limit)));

    (value, true)
}

#[cfg(test)]
mod test {
    use num::Complex;
    use serde_json::Value;
    use test::Bencher;

    use super::_escapes;

    #[test]
    fn test_escapes() {
        // Create input vector
        let point = json!({"re": 0.5, "im": 0.5 });
        let limit = json!(100);
        let inputs: Vec<Vec<Value>> = vec!(vec!(point), vec!(limit));

        let _escapes = super::escapes(inputs);
    }
}