use std::panic;
use runlist::Dispatch;
use runlist::Output;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;

pub fn looper(dispatch_rx: Receiver<Dispatch>, output_tx: Sender<Output>) {
    thread::spawn(move || {
        set_panic_hook();

        loop {
            match dispatch_rx.recv() {
                Ok(dispatch) => {
                    debug!("Received dispatch over channel");
                    match output_tx.send(execute(dispatch)) {
                        Err(_) => break,
                        _ => debug!("Returned Function Output over channel")
                    }
                },
                _ => break
            }
        }
    });
}

pub fn execute(dispatch: Dispatch) -> Output {
    // Run the implementation with the input values and catch the execution result
    let result = dispatch.implementation.run(dispatch.input_values.clone());

    return Output {
        id: dispatch.id,
        input_values: dispatch.input_values,
        result,
        destinations: dispatch.destinations,
    };
}

/*
    Replace the standard panic hook with one that just outputs the file and line of any process's
    runtime panic.
*/
fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        error!("{:?}", panic_info.payload());
        if let Some(location) = panic_info.location() {
            error!("panic occurred in file '{}' at line {}", location.file(), location.line());
        } else {
            error!("panic occurred but can't get location information");
        }
    }));
    debug!("Panic hook set to catch panics in process execution");
}