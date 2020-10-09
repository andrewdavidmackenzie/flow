use flow_impl::Implementation;
use flow_impl_derive::FlowImpl;
use serde_json::{json, Value};

/*
    Given the row and column of a pixel in the output image, return the
    corresponding point on the complex plane.

    `size` is a pair giving the width and height of the image in pixels.
    `pixel` is a (row, column) pair indicating a particular pixel in that image.
    `bounds` is two complex numbers - `upper_left` and `lower_right` designating the area our image covers.
*/
#[derive(FlowImpl, Debug)]
pub struct PixelToPoint;

impl PixelToPoint {
    fn pixel_to_point(upper_left: &Vec<Value>,
                      lower_right: &Vec<Value>,
                      pixel: &Vec<Value>,
                      size: &Vec<Value>) -> [f64; 2]
    {
        let width = lower_right[0].as_f64().unwrap() - upper_left[0].as_f64().unwrap(); // real
        let height = upper_left[1].as_f64().unwrap() - lower_right[1].as_f64().unwrap(); // imaginary

        let mut complex: [f64;2] = [0.0, 0.0];

        // This is subtraction as pixel[1] increases as we go down,
        // but the imaginary component increases as we go up.
        complex[0] = upper_left[0].as_f64().unwrap() +
            (pixel[0].as_f64().unwrap() as f64 * (width / size[0].as_f64().unwrap() as f64));
        complex[1] = upper_left[1].as_f64().unwrap() -
            (pixel[1].as_f64().unwrap() as f64 * (height / size[1].as_f64().unwrap() as f64));

        complex
    }
}

impl Implementation for PixelToPoint {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, bool) {
        let bounds = inputs[0].as_array().unwrap();
        let upper_left = bounds[0].as_array().unwrap();
        let lower_right = bounds[1].as_array().unwrap();
        let pixel = inputs[1].as_array().unwrap();
        let size = inputs[2].as_array().unwrap();

        let complex_point = Self::pixel_to_point(upper_left, lower_right, pixel, size);

        let result = Some(json!([pixel, complex_point]));

        (result, true)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::{json, Value};

    // bounds = inputs[0]
    //      upper_left = bounds[0];
    //      lower_right = bounds[1];
    // pixel = inputs[1]
    // size = inputs[2]
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
}


