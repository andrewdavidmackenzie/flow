use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct ReadSection;

impl Implementation for ReadSection {
    fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let input_stream = inputs.remove(0);
        let ra = input_stream[0].as_str().unwrap().parse::<u64>();
        let rb = input_stream[1].as_str().unwrap().parse::<u64>();
        let rc = input_stream[2].as_str().unwrap().parse::<u64>();

        match (ra, rb, rc) {
            (Ok(a), Ok(b), Ok(c)) => {
                let json = json!([a, b, c]);
                println!("json = {}", json.to_string());
                run_list.send_output(process, json);
            },
            _ => {}
        }

        true
    }
}