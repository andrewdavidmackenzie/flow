use flowmacro::flow_function;
use num::Complex;
use serde_json::{json, Value};

pub fn escapes(c_array: [f64; 2], limit: u64) -> u64 {
    let c = Complex {
        re: c_array[0], im: c_array[1]
    };

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

    let c: [f64; 2] = [point[0].as_f64().ok_or("Could not get as f64")?,
                    point[1].as_f64().ok_or("Could not get as f64")?];

    let value = escapes(c, 255);

    // Fake Grey via RGB for now
    let result = Some(json!([pixel, [value, value, value]]));

    Ok((result, RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use super::escapes;

    #[test]
    fn pixel() {
        let value = escapes([0.5, 0.5], 255);

        assert_eq!(4, value);
    }

    #[cfg(nightly)]
    mod bench_tests {
        extern crate test;

        use test::Bencher;

        use num::Complex;

        use super::escapes;

        #[bench]
        fn bench_escapes(b: &mut Bencher) {
            let upper_left = [-1.20, 0.35];

            b.iter(|| escapes(upper_left, 255));
        }
    }
}
