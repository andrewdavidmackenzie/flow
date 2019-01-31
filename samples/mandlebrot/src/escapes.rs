use num::Complex;
use serde_json::Value as JsonValue;

/*
    Try to determine if 'c' is in the Mandlebrot set, using at most 'limit' iterations to decide
    If 'c' is not a member, return 'Some(i)', where 'i' is the number of iterations it took for 'c'
    to leave the circle of radius two centered on the origin.
    If 'c' seems to be a member (more precisely, if we reached the iteration limit without being
    able to prove that 'c' is not a member) return 'None'
*/
#[no_mangle]
pub extern "C" fn escapes(mut inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, bool) {
    let point = inputs.remove(0).remove(0);
    // pixel_bounds: (usize, usize),
    let re = point["re"].as_f64().unwrap();
    let im = point["im"].as_f64().unwrap();
    let complex_point = Complex { re, im };

    let limit = inputs.remove(0).remove(0).as_u64().unwrap();

    let value = Some(json!(_escapes(complex_point, limit)));

    (value, true)
}

/// Try to determine if 'c' is in the Mandlebrot set, using at most 'limit' iterations to decide
/// If 'c' is not a member, return 'Some(i)', where 'i' is the number of iterations it took for 'c'
/// to leave the circle of radius two centered on the origin.
/// If 'c' seems to be a member (more precisely, if we reached the iteration limit without being
/// able to prove that 'c' is not a member) return 'None'
pub fn _escapes(c: Complex<f64>, limit: u64) -> u64 {
    if c.norm_sqr() > 4.0 {
        return 0;
    }

    let mut z = c;

    for i in 1..limit {
        z = z * z + c;
        if z.norm_sqr() > 4.0 {
            return i;
        }
    }

    return 255;
}

#[cfg(test)]
mod tests {
    use num::Complex;
    use serde_json::Value as JsonValue;
    use test::Bencher;

    use super::_escapes;

    #[test]
    fn test_escapes() {
        // Create input vector
        let point = json!({"re": 0.5, "im": 0.5 });
        let limit = json!(100);
        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(point), vec!(limit));

        let _escapes = super::escapes(inputs);
    }

    #[bench]
    fn bench_escapes(b: &mut Bencher) {
        let upper_left = Complex { re: -1.20, im: 0.35 };

        b.iter(|| _escapes(upper_left, 255));
    }
}