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
fn _escapes(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let pixel_point = inputs[0].as_array().ok_or("Could not get as array")?;

    let pixel = pixel_point[0].as_array().ok_or("Could not get as array")?;
    let point = pixel_point[1].as_array().ok_or("Could not get as array")?;

    let c = Complex {
        re: point[0].as_f64().ok_or("Could not get as f64")?,
        im: point[1].as_f64().ok_or("Could not get as f64")?,
    };

    let value = escapes(c, 255);

    // Fake Grey via RGB for now
    let result = Some(json!([pixel, [value, value, value]]));

    Ok((result, RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use super::_escapes;

    #[test]
    fn pixel() {
        let pixel_point = json!([[50, 50], [0.5, 0.5]]);

        let inputs: Vec<Value> = vec![pixel_point];
        let (results, _) = _escapes(&inputs).expect("_escapes() failed");

        let results_json = results.expect("No result returned");
        let results_array = results_json.as_array().expect("Could not get as array");

        let pixel = results_array[0].as_array().expect("Could not get as array");
        let value_array = results_array[1].as_array().expect("Could not get as array");
        let value = value_array[0].as_i64().expect("Could not get as i64") as u8;

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
