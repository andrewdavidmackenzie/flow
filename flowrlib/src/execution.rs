use process::Process;
use runlist::RunList;
use std::panic;
use std::sync::{Arc, Mutex};
use debug_client::DebugClient;

/// The generated code for a flow consists of values and functions formed into a list of Processs.
///
/// This list is built program start-up in `main` which then starts execution of the flow by calling
/// this `execute` method.
///
/// You should not have to write code to call `execute` yourself, it will be called from the
/// generated code in the `main` method.
///
/// On completion of the execution of the flow it will return and `main` will call `exit`
///
/// # Example
/// ```
/// use std::sync::{Arc, Mutex};
/// use std::io;
/// use std::io::Write;
/// use flowrlib::process::Process;
/// use flowrlib::execution::execute;
/// use std::process::exit;
/// use flowrlib::debug_client::DebugClient;
///
/// struct CLIDebugClient {}
///
/// impl DebugClient for CLIDebugClient {
///    fn display(&self, output: &str) {
///        print!("{}", output);
///        io::stdout().flush().unwrap();
///    }
///
///    fn read_input(&self, input: &mut String) -> io::Result<usize> {
///        io::stdin().read_line(input)
///    }
/// }
///
/// const CLI_DEBUG_CLIENT: &DebugClient = &CLIDebugClient{};
///
/// let mut processs = Vec::<Arc<Mutex<Process>>>::new();
///
/// execute(processs, false /* print_metrics */, CLI_DEBUG_CLIENT, false /* use_debugger */);
///
/// exit(0);
/// ```
pub fn execute(processes: Vec<Arc<Mutex<Process>>>, display_metrics: bool,
               client: &'static DebugClient, use_debugger: bool) {
    set_panic_hook();
    let mut run_list = RunList::new(client, use_debugger);
    run_list.run(processes);

    if display_metrics {
        #[cfg(feature = "metrics")]
        run_list.print_metrics();
        println!("\t\tProcess dispatches: \t{}\n", run_list.state.dispatches());
    }
}

/*
    Replace the standard panic hook with one that just outputs the file and line of any process's
    runtime panic.
*/
fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(location) = panic_info.location() {
            error!("panic occurred in file '{}' at line {}", location.file(), location.line());
        } else {
            error!("panic occurred but can't get location information...");
        }
    }));
    debug!("Panic hook set to catch panics in processs");
}