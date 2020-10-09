use num::Complex;
use pixel_to_point::pixel_to_point;
use rayon::prelude::*;
use render_a_row::render_a_row;

/*
    This function renders an area of mandelbrot set by row, where the algorithm across rows
    is parallelized using rayon. This is not used in the flow and so has no wrapping methods.
*/
pub fn render_by_row(pixels: &mut [u8], bounds: (usize, usize),
                 upper_left: Complex<f64>, lower_right: Complex<f64>) {
    // split array of bytes for bitmap into chunks the size of a row width
    let rows = pixels.par_chunks_mut(bounds.0);

    let row_bounds = (bounds.0, 1);

    // in parallel render each row into the chunk of memory for it
    rows.into_par_iter()
        .enumerate() // produces (row_index, row) --> Our mapper effectively
        .for_each(|(row_index, row)| {
            let row_upper_left = pixel_to_point(bounds, (0, row_index),
                                                upper_left, lower_right);
            let row_lower_right = pixel_to_point(bounds, (bounds.0, row_index),
                                                 upper_left, lower_right);
            render_a_row(row, row_bounds, row_index, row_upper_left, row_lower_right);
        });

    // When ends all rows have been rendered, as since the memory is already contiguous
    // there is no need for a reducer
}