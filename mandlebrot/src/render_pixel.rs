use escapes::escapes;
use num::Complex;
use pixel_to_point::pixel_to_point;

/*
    This implementation is for the code version only. In the flow version this is broken down
    further in a flow (render_pixel.toml) that uses the same functions as called here.
*/
pub fn render_pixel(row_bounds: (usize, usize),
                    pixel: (usize, usize),
                    upper_left: Complex<f64>,
                    lower_right: Complex<f64>,
                    limit: u64) -> u8 {
    let point = pixel_to_point(row_bounds, pixel, upper_left, lower_right);

    255 - escapes(point, limit) as u8
}