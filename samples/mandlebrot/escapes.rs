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
    fn run(&self, runnable: &Runnable, mut inputs: Vec<JsonValue>, run_list: &mut RunList) {
        let point = inputs.remove(0);
        // pixel_bounds: (usize, usize),
        let re = point["re"].as_f64().unwrap();
        let im = point["im"].as_f64().unwrap();
        let c = Complex { re, im };

        let limit = inputs.remove(0).as_u64().unwrap();

        if c.norm_sqr() > 4.0 {
            run_list.send_output(runnable, json!(255));
            return;
        }

        let mut z = c;

        for i in 1..limit {
            z = z * z + c;
            if z.norm_sqr() > 4.0 {
                let output = json!(i);
                run_list.send_output(runnable, output);
                return;
            }
        }

        run_list.send_output(runnable, json!(255));
    }
}

#[cfg(test)]
mod tests {
    use flowrlib::runnable::Runnable;
    use flowrlib::runlist::RunList;
    use flowrlib::function::Function;
    use serde_json::Value as JsonValue;
    use super::Escapes;

    #[test]
    fn escapes() {
        // Create input vector
        let point = json!({"re": 0.5, "im": 0.5 });
        let limit = json!(100);
        let inputs: Vec<JsonValue> = vec!(point, limit);

        let mut run_list = RunList::new();
        let escapes = &Function::new("escapes".to_string(), 3, 0, Box::new(Escapes), None, vec!()) as &Runnable;
        let implementation = escapes.implementation();

        implementation.run(escapes, inputs, &mut run_list);
    }
}