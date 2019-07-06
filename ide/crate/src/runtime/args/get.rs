use flowrlib::implementation::DONT_RUN_AGAIN;
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use serde_json::Value;

pub struct Get;

impl Implementation for Get {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let args_el = document.get_element_by_id("args").expect("could not find 'stdout' element");
        let args_text = args_el.inner_html();

        let flow_args: Vec<&str> = args_text.split(' ').collect();

        (Some(json!(flow_args)), DONT_RUN_AGAIN)
    }
}