use flowrlib::function::Function;

use flowstdlib::stdio::stdout::Stdout;

pub fn get_functions() -> Vec<Box<Function>> {
    let mut functions = Vec::<Box<Function>>::with_capacity(1);

    functions.push(Box::new(Function::new(&Stdout{}, 0)));

    functions
}