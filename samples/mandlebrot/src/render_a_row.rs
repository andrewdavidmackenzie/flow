
use num::Complex;
use render_pixel::render_pixel;

/// Render a row of the Mandlebrot set into a buffer of pixels
/// The 'bounds' argument gives the width and height of the buffer 'pixels' which holds one
/// grayscale pixel per byte. The 'upper_left' and 'lower_right' arguments specify points on the
/// complex plane corresponding to the upper left and lower right corners of the pixel buffer.
pub fn render_a_row(pixels: &mut [u8], row_bounds: (usize, usize), row_index: usize,
                    row_upper_left: Complex<f64>, row_lower_right: Complex<f64>) {
    let mut offset: usize = 0;

    for column in 0..row_bounds.0 {
        pixels[offset] = render_pixel(row_bounds, (column, row_index), row_upper_left, row_lower_right, 255) as u8;
        offset += 1; // move forward a byte in the pixel buffer
    }
}