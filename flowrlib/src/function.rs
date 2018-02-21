use runnable::Runnable;
use implementation::Implementation;
use std::mem::replace;
use std::fmt;

pub struct Function {
    id: usize,
    implementation: Box<Implementation>,

    num_inputs: usize,
    num_inputs_pending: usize,
    inputs: Vec<Option<String>>,

    output_routes: Vec<(usize, usize)>,
}

// TODO these methods will need to be made thread safe

// TODO Make these doc comments and produce some documentation?

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tid: {}\n\timplementation: {}\n",
               self.id, self.implementation.name()).unwrap();
        write!(f, "\tnum_inputs: {:?}\n\tnum_inputs_pending: {:?}\n",
               self.num_inputs, self.num_inputs_pending).unwrap();
        write!(f, "\tinputs: {:?}\n\toutput_routes: {:?}\n",
               self.inputs, self.output_routes).unwrap();
        Ok(())
    }
}

impl Function {
    pub fn new(id: usize, implementation: Box<Implementation>,
               output_routes: Vec<(usize, usize)>)
               -> Function {
        let number_of_inputs = implementation.number_of_inputs();
        Function {
            id,
            implementation,
            num_inputs: number_of_inputs,
            num_inputs_pending: number_of_inputs,
            inputs: vec![None; number_of_inputs],
            output_routes,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Function;
    use super::Implementation;
    use runnable::Runnable;
    use std::fmt;
    use std::fmt::Debug;

    pub struct Stdout;

    impl Implementation for Stdout {
        fn number_of_inputs(&self) -> usize {
            1
        }

        fn run(&self, mut inputs: Vec<Option<String>>) -> Option<String> {
            println!("{:?}", inputs.remove(0).unwrap());
            None
        }

        fn name(&self) -> &'static str {
            "Stdout"
        }
    }

    impl Debug for Stdout {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "stdout defined in file: '{}'", file!())
        }
    }

    #[test]
    fn function_to_code() {
        let function = Function::new(1, Box::new(Stdout), vec!());
        let code = function.to_code();
        assert_eq!(code, "Function::new(1, Box::new(Stdout{}), vec!())")
    }
}

impl Runnable for Function {
    fn id(&self) -> usize { self.id }

    // If a function has zero inputs it is considered ready to run at init
    fn init(&mut self) -> bool {
        self.num_inputs == 0
    }

    fn write_input(&mut self, input_number: usize, input_value: Option<String>) {
        self.num_inputs_pending -= 1;
        self.inputs[input_number] = input_value;
    }

    // responds true if all inputs have been satisfied - false otherwise
    fn inputs_satisfied(&self) -> bool {
        self.num_inputs_pending == 0
    }

    /*
        Consume the inputs, reset the number of pending inputs and run the implementation
    */
    fn run(&mut self) -> Option<String> {
        let inputs = replace(&mut self.inputs, vec![None; self.num_inputs]);
        self.num_inputs_pending = self.num_inputs;
        info!("Running implementation: '{}'", &self.implementation.name());
        self.implementation.run(inputs)
    }

    fn output_destinations(&self) -> Vec<(usize, usize)> {
        self.output_routes.clone()
    }

    // example "Function::new(Box::new(Stdout{}), vec!())
    fn to_code(&self) -> String {
        let mut code = format!("Function::new({}, Box::new({}{{}}),", self.id,
                               self.implementation.name());

        // Add the vector of tuples of elements and their inputs it's connected to
        code.push_str(" vec!(");
        for ref route in &self.output_routes {
            code.push_str(&format!("({},{}),", route.0, route.1));
        }
        code.push_str(")");

        code.push_str(")");

        code
    }
}
