use num::Complex;
use serde_json::Value as JsonValue;

/*
    Given the row and column of a pixel in the output image, return the
    corresponding point on the complex plane.

    `bounds` is a pair giving the width and height of the image in pixels.
    `pixel` is a (row, column) pair indicating a particular pixel in that image.
    The `upper_left` and `lower_right` parameters are points on the complex
    plane designating the area our image covers.
*/
#[no_mangle]
pub extern "C" fn pixel_to_point(mut inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, bool) {
    let pixel_bounds = inputs.remove(0).remove(0);
    // pixel_bounds: (usize, usize),
    let pixel_bounds_x = pixel_bounds["x"].as_u64().unwrap() as usize;
    let pixel_bounds_y = pixel_bounds["y"].as_u64().unwrap() as usize;

    let complex_bounds = inputs.remove(0).remove(0);
    // complex_bounds(upper_left, lower_right): (Complex<f64>, Complex<f64>)
    let upper_left = &complex_bounds["ul"];
    let upper_left_re = upper_left["re"].as_f64().unwrap();
    let upper_left_im = upper_left["im"].as_f64().unwrap();
    let upper_left_complex = Complex { re: upper_left_re, im: upper_left_im };

    let lower_right = &complex_bounds["lr"];
    let lower_right_re = lower_right["re"].as_f64().unwrap();
    let lower_right_im = lower_right["im"].as_f64().unwrap();
    let lower_right_complex = Complex { re: lower_right_re, im: lower_right_im };

    let pixel = inputs.remove(0).remove(0);
    //pixel: (x, y),
    let pixel_x = pixel["x"].as_u64().unwrap() as usize;
    let pixel_y = pixel["y"].as_u64().unwrap() as usize;

    let complex_point = _pixel_to_point(
        (pixel_bounds_x, pixel_bounds_y),
        (pixel_x, pixel_y),
        upper_left_complex,
        lower_right_complex,
    );

    // output: Complex<f64>
    let value = Some(json!({ "re" : complex_point.re, "im": complex_point.im }));

    (value, true)
}

#[cfg(test)]
mod tests {
    use num::Complex;
    use serde_json::Value as JsonValue;

    #[test]
    fn pixel() {
        // Create input vector
        let pixel_bounds = json!({"x": 100, "y": 100 });
        let complex_bounds = json!({ "ul" : {"re": 0.0, "im": 0.0 }, "lr": {"re": 1.0, "im": 1.0 }});
        let pixel = json!({"x": 50, "y": 50 });
        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(pixel_bounds), vec!(complex_bounds), vec!(pixel));

        let _point = super::pixel_to_point(inputs);
    }
}


