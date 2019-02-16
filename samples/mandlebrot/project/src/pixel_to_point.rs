use num::Complex;

/// Given the row and column of a pixel in the output image, return the
/// corresponding point on the complex plane.
///
/// `bounds` is a pair giving the width and height of the image in pixels.
/// `pixel` is a (row, column) pair indicating a particular pixel in that image.
/// The `upper_left` and `lower_right` parameters are points on the complex
/// plane designating the area our image covers.
pub fn _pixel_to_point(bounds: (usize, usize), pixel: (usize, usize),
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
    use num::Complex;

    use super::_pixel_to_point;

    #[test]
    fn test_pixel_to_point() {
        let upper_left = Complex { re: -1.0, im: 1.0 };
        let lower_right = Complex { re: 1.0, im: -1.0 };

        assert_eq!(_pixel_to_point((100, 100), (25, 75),
                                   upper_left, lower_right),
                   Complex { re: -0.5, im: -0.5 });
    }
}


