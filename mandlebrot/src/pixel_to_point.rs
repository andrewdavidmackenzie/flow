use flowrlib::implementation::Implementation;
use flowrlib::runlist::RunList;
use flowrlib::runnable::Runnable;
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
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let width = inputs.remove(0).remove(0).as_u64().unwrap() as usize;
        let height = inputs.remove(0).remove(0).as_u64().unwrap() as usize;

        let upper_left = inputs.remove(0).remove(0);
        let upper_left_re = upper_left["re"].as_f64().unwrap();
        let upper_left_im = upper_left["im"].as_f64().unwrap();
        let upper_left_complex = Complex { re: upper_left_re, im: upper_left_im };

        let lower_right = inputs.remove(0).remove(0);
        let lower_right_re = lower_right["re"].as_f64().unwrap();
        let lower_right_im = lower_right["im"].as_f64().unwrap();
        let lower_right_complex = Complex { re: lower_right_re, im: lower_right_im };

        let pixel_x = inputs.remove(0).remove(0).as_u64().unwrap() as usize;
        let pixel_y = inputs.remove(0).remove(0).as_u64().unwrap() as usize;

        let complex_point = pixel_to_point(
            (width, height),
            (pixel_x, pixel_y),
            upper_left_complex,
            lower_right_complex,
        );

        // output: Complex<f64>
        // TODO Implement to_json() or similar for Complex? Or see if can just use serde and it
        // either already implements or we can derive serialization and deserialization
        let output = json!({ "re" : complex_point.re, "im": complex_point.im });
        run_list.send_output(runnable, output);

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
    use flowrlib::function::Function;
    use flowrlib::runlist::RunList;
    use flowrlib::runnable::Runnable;
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
        let width = json!(100);
        let height = json!(100);

        let upper_left = json!({"re": 0.0, "im": 0.0 });
        let lower_right = json!({"re": 1.0, "im": 1.0 });

        let pixel_x = json!(50);
        let pixel_y = json!(50);
        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(width), vec!(height), vec!(upper_left), vec!(lower_right), vec!(pixel_x), vec!(pixel_y));

        let mut run_list = RunList::new();
        let p2p = &Function::new("p2p", 3, vec!(1, 1, 1), 0, Box::new(PixelToPoint), None, vec!()) as &Runnable;
        let implementation = p2p.implementation();

        implementation.run(p2p, inputs, &mut run_list);
    }
}


