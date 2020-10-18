use flow_impl::Implementation;
use flow_impl_derive::FlowImpl;
use num::Complex;
use serde_json::{json, Value};

pub fn pixel_to_point(size: (usize, usize),
                  pixel: (usize, usize),
                  upper_left: Complex<f64>,
                  lower_right: Complex<f64>) -> Complex<f64> {
    let width = lower_right.re - upper_left.re;
    let height = upper_left.im - lower_right.im;

    Complex {
        re: upper_left.re + (pixel.0 as f64 * (width / size.0 as f64)),
        im: upper_left.im - (pixel.1 as f64 * (height / size.1 as f64)),
        // This is subtraction as pixel.1 increases as we go down,
        // but the imaginary component increases as we go up.
    }
}

/// Given the row and column of a pixel in the output image, return the
/// corresponding point on the complex plane.
///
/// `bounds` is a pair giving the width and height of the image in pixels.
/// `pixel` is a (row, column) pair indicating a particular pixel in that image.
/// The `upper_left` and `lower_right` parameters are points on the complex
/// plane designating the area our image covers.
#[derive(FlowImpl, Debug)]
pub struct PixelToPoint;

impl Implementation for PixelToPoint {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, bool) {
        let bounds = inputs[0].as_array().unwrap();
        let upper_left = bounds[0].as_array().unwrap();
        let upper_left_c = Complex {
            re: upper_left[0].as_f64().unwrap() as f64,
            im: upper_left[1].as_f64().unwrap() as f64,
        };

        let lower_right = bounds[1].as_array().unwrap();
        let lower_right_c = Complex {
            re: lower_right[0].as_f64().unwrap() as f64,
            im: lower_right[1].as_f64().unwrap() as f64,
        };

        let pixel = inputs[1].as_array().unwrap();
        let x = pixel[0].as_i64().unwrap() as usize;
        let y = pixel[1].as_i64().unwrap() as usize;

        let size = inputs[2].as_array().unwrap();
        let width = size[0].as_i64().unwrap() as usize;
        let height = size[1].as_i64().unwrap() as usize;

        let complex_point = pixel_to_point((width, height), // size
                                                 (x, y), // pixel
                                                 upper_left_c,
                                                 lower_right_c);

        let result = Some(json!([pixel, [complex_point.re, complex_point.im]]));

        (result, true)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use num::Complex;
    use serde_json::{json, Value};
    use wasm_bindgen_test::*;

    // bounds = inputs[0]
    //      upper_left = bounds[0];
    //      lower_right = bounds[1];
    // pixel = inputs[1]
    // size = inputs[2]
    #[wasm_bindgen_test]
    #[test]
    fn pixel() {
        // Create input vector
        let bounds = json!([[0.0, 0.0], [1.0, 1.0]]);
        let pixel = json!([50, 50]);
        let size = json!([100, 100]);

        let inputs: Vec<Value> = vec!(bounds, pixel, size);

        let pixelator = super::PixelToPoint {};
        let (result, _) = pixelator.run(&inputs);

        let result_json = result.unwrap();
        let results = result_json.as_array().unwrap();

        let pixel = results[0].as_array().unwrap();
        let point = results[1].as_array().unwrap();

        assert_eq!(50, pixel[0]);
        assert_eq!(50, pixel[1]);
        assert_eq!(0.5, point[0]);
        assert_eq!(0.5, point[1]);
    }

    #[test]
    fn test_pixel_to_point() {
        let upper_left = Complex { re: -1.0, im: 1.0 };
        let lower_right = Complex { re: 1.0, im: -1.0 };

        assert_eq!(super::pixel_to_point((100, 100), (25, 75),
                                   upper_left, lower_right),
                   Complex { re: -0.5, im: -0.5 });
    }
}


