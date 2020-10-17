use num::Complex;

/// Try to determine if 'c' is in the Mandlebrot set, using at most 'limit' iterations to decide
/// If 'c' is not a member, return 'Some(i)', where 'i' is the number of iterations it took for 'c'
/// to leave the circle of radius two centered on the origin.
/// If 'c' seems to be a member (more precisely, if we reached the iteration limit without being
/// able to prove that 'c' is not a member) return 'None'
pub fn _escapes(c: Complex<f64>, limit: u64) -> u64 {
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

#[cfg(test)]
mod test {
    extern crate test;

    use num::Complex;
    use test::Bencher;

    use super::_escapes;

    #[bench]
    fn bench_escapes(b: &mut Bencher) {
        let upper_left = Complex { re: -1.20, im: 0.35 };

        b.iter(|| _escapes(upper_left, 255));
    }
}