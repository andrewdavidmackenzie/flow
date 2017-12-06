use flowrlib::function::Function;
use flowstdlib::stdio::stdout::Stdout;

// Use alias for the instance name?
static this_stdout: Stdout = Stdout{};

pub static functions: [&'static (Function+Sync); 1] = [&this_stdout];