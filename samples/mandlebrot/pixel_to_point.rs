use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct PixelToPoint;

/*
    Given the row and column of a pixel in the output image, return the
    corresponding point on the complex plane.

    `bounds` is a pair giving the width and height of the image in pixels.
    `pixel` is a (row, column) pair indicating a particular pixel in that image.
    The `upper_left` and `lower_right` parameters are points on the complex
    plane designating the area our image covers.
*/
impl Implementation for PixelToPoint {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<JsonValue>, run_list: &mut RunList) {
        let pixel_bounds = inputs.remove(0);
        // pixel_bounds: (usize, usize),
        let pixel_bounds_x = pixel_bounds["x"].as_f64().unwrap();
        let pixel_bounds_y = pixel_bounds["y"].as_f64().unwrap();

        let complex_bounds = inputs.remove(0);
        // complex_bounds(upper_left, lower_right): (Complex<f64>, Complex<f64>)
        let upper_left = &complex_bounds["ul"];
        let upper_left_re = upper_left["re"].as_f64().unwrap();
        let upper_left_im = upper_left["im"].as_f64().unwrap();

        let lower_right = &complex_bounds["lr"];
        let lower_right_re = lower_right["re"].as_f64().unwrap();
        let lower_right_im = lower_right["im"].as_f64().unwrap();

        let pixel = inputs.remove(0);
        //pixel: (x, y),
        let pixel_x = pixel["x"].as_f64().unwrap();
        let pixel_y = pixel["y"].as_f64().unwrap();

        let complex_width = lower_right_re - upper_left_re;
        let complex_height = upper_left_im - lower_right_im;

        // This is subtraction as pixel.1 increases as we go down,
        // but the imaginary component increases as we go up.
        let re = upper_left_re + (pixel_x as f64 * (complex_width / pixel_bounds_x as f64));
        let im = upper_left_im - (pixel_y as f64 * (complex_height / pixel_bounds_y as f64));

        // output: Complex<f64>
        let output = json!({ "re" : re, "im": im });
        run_list.send_output(runnable, output);
    }
}

#[cfg(test)]
mod tests {
    use flowrlib::runnable::Runnable;
    use flowrlib::runlist::RunList;
    use flowrlib::function::Function;
    use serde_json::Value as JsonValue;
    use super::PixelToPoint;

    #[test]
    fn pixel() {
        // Create input vector
        let pixel_bounds = json!({"x": 100, "y": 100 });
        let complex_bounds = json!({ "ul" : {"re": 0.0, "im": 0.0 }, "lr": {"re": 1.0, "im": 1.0 }});
        let pixel = json!({"x": 50, "y": 50 });
        let inputs: Vec<JsonValue> = vec!(pixel_bounds, complex_bounds, pixel);

        let mut run_list = RunList::new();
        let p2p = &Function::new("p2p".to_string(), 3, 0, Box::new(PixelToPoint), None, vec!()) as &Runnable;
        let implementation = p2p.implementation();

        implementation.run(p2p, inputs, &mut run_list);
    }
}


