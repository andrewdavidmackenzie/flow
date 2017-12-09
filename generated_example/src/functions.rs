use flowrlib::function::Function;

use flowstdlib::stdio::stdout::Stdout;

pub fn get_functions() -> Vec<Function> {
    let mut functions = Vec::<Function>::with_capacity(1);

    functions.push(Function::new(&Stdout{}, 0));

    functions
}