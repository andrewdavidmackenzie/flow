use flow_impl::Implementation;
use flow_impl_derive::FlowImpl;
use num::Complex;
use serde_json::{json, Value};

/*
    Given the row and column of a pixel in the output image, return the
    corresponding point on the complex plane.

    `size` is a pair giving the width and height of the image in pixels.
    `pixel` is a (row, column) pair indicating a particular pixel in that image.
    `bounds` is two complex numbers - `upper_left` and `lower_right` designating the area our image covers.
*/
#[derive(FlowImpl, Debug)]
pub struct RenderPixel;

impl RenderPixel {
    /// Try to determine if 'c' is in the Mandlebrot set, using at most 'limit' iterations to decide
    /// If 'c' is not a member, return 'Some(i)', where 'i' is the number of iterations it took for 'c'
    /// to leave the circle of radius two centered on the origin.
    /// If 'c' seems to be a member (more precisely, if we reached the iteration limit without being
    /// able to prove that 'c' is not a member) return 'None'
    fn escapes(complex: &Vec<Value>, limit: u64) -> u64 {
        let c = Complex {
            re: complex[0].as_f64().unwrap(),
            im: complex[1].as_f64().unwrap()
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

        return 255;
    }
}

impl Implementation for RenderPixel {
    /*
        Try to determine if 'c' is in the Mandlebrot set, using at most 'limit' iterations to decide
        If 'c' is not a member, return 'Some(i)', where 'i' is the number of iterations it took for 'c'
        to leave the circle of radius two centered on the origin.
        If 'c' seems to be a member (more precisely, if we reached the iteration limit without being
        able to prove that 'c' is not a member) return 'None'
    */
    fn run(&self, inputs: &[Value]) -> (Option<Value>, bool) {
        let pixel = inputs[0].as_array().unwrap();
        let point = inputs[1].as_array().unwrap();

        let value = Self::escapes(point, 255);

        let result = Some(json!([pixel, [value]]));

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
        let pixel = json!([50, 50]);
        let point = json!([0.5, 0.5]);

        let inputs: Vec<Value> = vec!(pixel, point);

        let renderer = super::RenderPixel {};
        let (result, _) = renderer.run(&inputs);

        let result_json = result.unwrap();
        let results = result_json.as_array().unwrap();

        let pixel = results[0].as_array().unwrap();
        let value_array = results[1].as_array().unwrap();
        let value = value_array[0].as_i64().unwrap() as u8;

        assert_eq!(50, pixel[0]);
        assert_eq!(50, pixel[1]);
        assert_eq!(4, value);
    }
}