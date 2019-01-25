use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use num::Complex;
use serde_json::Value as JsonValue;

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
    fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
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

        let complex_point = pixel_to_point(
            (pixel_bounds_x, pixel_bounds_y),
            (pixel_x, pixel_y),
            upper_left_complex,
            lower_right_complex,
        );

        // output: Complex<f64>
        let output = json!({ "re" : complex_point.re, "im": complex_point.im });
        run_list.send_output(process, output);

        true
    }
}


/// Given the row and column of a pixel in the output image, return the
/// corresponding point on the complex plane.
///
/// `bounds` is a pair giving the width and height of the image in pixels.
/// `pixel` is a (row, column) pair indicating a particular pixel in that image.
/// The `upper_left` and `lower_right` parameters are points on the complex
/// plane designating the area our image covers.
pub fn pixel_to_point(bounds: (usize, usize), pixel: (usize, usize),
                      upper_left: Complex<f64>,
                      lower_right: Complex<f64>) -> Complex<f64>
{
    let width = lower_right.re - upper_left.re;
    let height = upper_left.im - lower_right.im;

    Complex {
        re: upper_left.re + (pixel.0 as f64 * (width / bounds.0 as f64)),
        im: upper_left.im - (pixel.1 as f64 * (height / bounds.1 as f64)),
        // This is subtraction as pixel.1 increases as we go down,
        // but the imaginary component increases as we go up.
    }
}

#[cfg(test)]
mod tests {
    use flowrlib::process::Process;
    use flowrlib::runlist::RunList;
    use num::Complex;
    use serde_json::Value as JsonValue;

    use super::pixel_to_point;
    use super::PixelToPoint;

    #[test]
    fn test_pixel_to_point() {
        let upper_left = Complex { re: -1.0, im: 1.0 };
        let lower_right = Complex { re: 1.0, im: -1.0 };

        assert_eq!(pixel_to_point((100, 100), (25, 75),
                                  upper_left, lower_right),
                   Complex { re: -0.5, im: -0.5 });
    }

    #[test]
    fn pixel() {
        // Create input vector
        let pixel_bounds = json!({"x": 100, "y": 100 });
        let complex_bounds = json!({ "ul" : {"re": 0.0, "im": 0.0 }, "lr": {"re": 1.0, "im": 1.0 }});
        let pixel = json!({"x": 50, "y": 50 });
        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(pixel_bounds), vec!(complex_bounds), vec!(pixel));

        let mut run_list = RunList::new();
        let p2p = &Function::new("p2p", 3, true, vec!(1, 1, 1), 0, Box::new(PixelToPoint), None, vec!()) as &Process;
        let implementation = p2p.implementation();

        implementation.run(p2p, inputs, &mut run_list);
    }
}


