pub mod validate;
mod run;

use clap::App;

pub fn register<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    let app = validate::register(app);
    run::register(app)
}
