use runnable::Runnable;
use implementation::Implementation;
use std::mem::replace;

#[derive(Debug)]
pub struct Function {
    initial_value: Option<&'static str>,
    implementation: Box<Implementation>,

    num_inputs: usize,
    num_inputs_pending: usize,
    inputs: Vec<Option<String>>,

    output_routes: Vec<(usize, usize)>
}

// TODO these methods will need to be made thread safe

// TODO Make these doc comments and produce some documentation?

impl Function {
    pub fn new(implementation: Box<Implementation>,
               output_routes: Vec<(usize, usize)>)
               -> Function {
        let number_of_inputs = implementation.number_of_inputs();
        Function {
            initial_value: None,
            implementation,
            num_inputs: number_of_inputs,
            num_inputs_pending: number_of_inputs,
            inputs: vec![None; number_of_inputs],
            output_routes
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

    const DEFINITION: &'static str ="
name = 'Stdout'
[[input]]
name = 'stdout'
type = 'String'";

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

        fn define(&self) -> &'static str {
            DEFINITION
        }
    }

    impl Debug for Stdout {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "stdout defined in file: '{}'", file!())
        }
    }

    #[test]
    fn function_to_code() {
        let function = Function::new(Box::new(Stdout), vec!());
        let code = function.to_code();
        assert_eq!(code, "Function::new(Box::new(Stdout{}), vec!())")
    }
}


impl Runnable for Function {
    fn init(&mut self) -> bool { false }

    /*
        provide a given input
    */
    fn write_input(&mut self, input_number: usize, input_value: Option<String>) -> bool {
        self.num_inputs_pending -= 1;
        self.inputs[input_number] = input_value;
        self.num_inputs_pending == 0 // all inputs satisfied
    }

    fn run(&mut self) -> Option<String> {
        // Consume the inputs
        let inputs = replace(&mut self.inputs, vec![None; self.num_inputs]);
        self.implementation.run(inputs)
    }

    fn get_affected(&self) -> Vec<(usize, usize)> {
        self.output_routes.clone()
    }

    // example "Function::new(Box::new(Stdout{}), vec!())
    fn to_code(&self) -> String {
        let mut code = format!("Function::new(Box::new({}{{}}),", self.implementation.name());

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
