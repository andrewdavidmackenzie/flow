use flow_macro::flow_function;
use num::Complex;
use serde_json::{json, Value};

pub fn escapes(c: Complex<f64>, limit: u64) -> u64 {
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

    limit
}

#[flow_function]
fn _escapes(inputs: &[Value]) -> (Option<Value>, bool) {
    let pixel_point = inputs[0].as_array().unwrap();

    let pixel = pixel_point[0].as_array().unwrap();
    let point = pixel_point[1].as_array().unwrap();

    let c = Complex {
        re: point[0].as_f64().unwrap(),
        im: point[1].as_f64().unwrap(),
    };

    let value = escapes(c, 255);

    // Fake Grey via RGB for now
    let result = Some(json!([pixel, [value, value, value]]));

    (result, true)
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};
    use super::_escapes;

    // bounds = inputs[0]
    //      upper_left = bounds[0];
    //      lower_right = bounds[1];
    // pixel = inputs[1]
    // size = inputs[2]
    #[test]
    fn pixel() {
        let pixel_point = json!([[50, 50], [0.5, 0.5]]);

        let inputs: Vec<Value> = vec![pixel_point];
        let (result, _) = _escapes(&inputs);

        let result_json = result.unwrap();
        let results = result_json.as_array().unwrap();

        let pixel = results[0].as_array().unwrap();
        let value_array = results[1].as_array().unwrap();
        let value = value_array[0].as_i64().unwrap() as u8;

        assert_eq!(50, pixel[0]);
        assert_eq!(50, pixel[1]);
        assert_eq!(4, value);
    }

    #[cfg(nightly)]
    mod bench_tests {
        extern crate test;

        use num::Complex;
        use test::Bencher;

        use super::escapes;

        #[bench]
        fn bench_escapes(b: &mut Bencher) {
            let upper_left = Complex {
                re: -1.20,
                im: 0.35,
            };

            b.iter(|| escapes(upper_left, 255));
        }
    }
}
