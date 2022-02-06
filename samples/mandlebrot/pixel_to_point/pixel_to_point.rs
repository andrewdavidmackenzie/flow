use flow_macro::flow_function;
use serde_json::{json, Value};

pub fn pixel_to_point(
    size: [usize; 2],
    pixel: [usize; 2],
    upper_left: [f64; 2],
    lower_right: [f64; 2],
) -> [f64;2] {
    let complex_width = lower_right[0] - upper_left[0];
    let complex_height = upper_left[1] - lower_right[1];

    [upper_left[0] + (pixel[0] as f64 * (complex_width / size[0] as f64)),
    // subtraction as pixel.1 increases as we go down, but the imaginary component increases as we go up.
     upper_left[1] - (pixel[1] as f64 * (complex_height / size[1] as f64))]
}

#[flow_function]
fn pixel_run(inputs: &[Value]) -> (Option<Value>, bool) {
    let bounds = inputs[0].as_array().unwrap();

    let upper_left = bounds[0].as_array().unwrap();
    let upper_left_c = [upper_left[0].as_f64().unwrap() as f64,
                                upper_left[1].as_f64().unwrap() as f64];

    let lower_right = bounds[1].as_array().unwrap();
    let lower_right_c = [lower_right[0].as_f64().unwrap() as f64,
                                lower_right[1].as_f64().unwrap() as f64];

    let pixel = inputs[1].as_array().unwrap();
    let x = pixel[0].as_i64().unwrap() as usize;
    let y = pixel[1].as_i64().unwrap() as usize;

    let size = inputs[2].as_array().unwrap();
    let width = size[0].as_i64().unwrap() as usize;
    let height = size[1].as_i64().unwrap() as usize;

    let complex_point = pixel_to_point(
        [width, height], // size
        [x, y],          // pixel
        upper_left_c,
        lower_right_c,
    );

    let result = Some(json!([pixel, complex_point]));

    (result, true)
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};
    use super::pixel_run;

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

        let inputs: Vec<Value> = vec![bounds, pixel, size];

        let (result, _) = pixel_run(&inputs);

        let result_json = result.unwrap();
        let results = result_json.as_array().unwrap();

        let pixel = results[0].as_array().unwrap();
        let point = results[1].as_array().unwrap();

        assert_eq!(50, pixel[0]);
        assert_eq!(50, pixel[1]);
        assert!((0.5 - point[0].as_f64().unwrap()).abs() < f64::EPSILON);
        assert!((0.5 - point[1].as_f64().unwrap()).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pixel_to_point() {
        let upper_left = [-1.0, 1.0];
        let lower_right = [1.0, -1.0];

        assert_eq!(
            super::pixel_to_point([100, 100], [25, 75], upper_left, lower_right),
            [-0.5, -0.5 ]
        );
    }
}
