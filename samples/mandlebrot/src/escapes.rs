use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;
use num::Complex;

pub struct Escapes;

/*
    Try to determine if 'c' is in the Mandlebrot set, using at most 'limit' iterations to decide
    If 'c' is not a member, return 'Some(i)', where 'i' is the number of iterations it took for 'c'
    to leave the circle of radius two centered on the origin.
    If 'c' seems to be a member (more precisely, if we reached the iteration limit without being
    able to prove that 'c' is not a member) return 'None'
*/
impl Implementation for Escapes {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let point = inputs.remove(0).remove(0);
        // pixel_bounds: (usize, usize),
        let re = point["re"].as_f64().unwrap();
        let im = point["im"].as_f64().unwrap();
        let complex_point = Complex { re, im };

        let limit = inputs.remove(0).remove(0).as_u64().unwrap();

        run_list.send_output(runnable, json!(escapes(complex_point, limit)));

        true
    }
}

/// Try to determine if 'c' is in the Mandlebrot set, using at most 'limit' iterations to decide
/// If 'c' is not a member, return 'Some(i)', where 'i' is the number of iterations it took for 'c'
/// to leave the circle of radius two centered on the origin.
/// If 'c' seems to be a member (more precisely, if we reached the iteration limit without being
/// able to prove that 'c' is not a member) return 'None'
pub fn escapes(c: Complex<f64>, limit: u64) -> u64 {
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
mod tests {
    use flowrlib::runnable::Runnable;
    use flowrlib::runlist::RunList;
    use flowrlib::function::Function;
    use serde_json::Value as JsonValue;
    use super::Escapes;
    use super::escapes;
    use test::Bencher;
    use num::Complex;

    #[test]
    fn test_escapes() {
        // Create input vector
        let point = json!({"re": 0.5, "im": 0.5 });
        let limit = json!(100);
        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(point), vec!(limit));

        let mut run_list = RunList::new();
        let escapes = &Function::new("escapes", 2, true, vec!(1,1), 0, Box::new(Escapes), None, vec!()) as &Runnable;
        let implementation = escapes.implementation();

        implementation.run(escapes, inputs, &mut run_list);
    }

    #[bench]
    fn bench_escapes(b: &mut Bencher) {
        let upper_left = Complex { re: -1.20, im: 0.35 };

        b.iter(|| escapes(upper_left, 255));
    }
}