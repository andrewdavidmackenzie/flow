use serde_json::Value as JsonValue;
use super::runlist::RunList;
use std::sync::Arc;

pub trait Runnable {
    fn name(&self) -> &str;
    fn number_of_inputs(&self) -> usize;
    fn id(&self) -> usize;
    fn init(&mut self) -> bool;
    fn write_input(&mut self, input_number: usize, new_value: JsonValue);
    fn inputs_satisfied(&self) -> bool;
    fn run(&mut self) -> JsonValue;
    fn output_destinations(&self) -> &Vec<(& 'static str, usize, usize)>;
    fn execute(&mut self, run_list: &mut RunList) {
        let output = self.run();

        if output != JsonValue::Null {
            debug!("\tProcessing output of runnable: #{} '{}'", self.id(), self.name());
            self.process_output(run_list, output);
        }
    }

    fn process_output(&mut self, run_list: &mut RunList, output: JsonValue) {
        for &(output_route, destination_id, io_number) in self.output_destinations() {
            let destination_arc = Arc::clone(&run_list.runnables[destination_id]);
            let mut destination = destination_arc.lock().unwrap();
            let output_value = output.pointer(output_route).unwrap();
            debug!("\tSending output '{}' from runnable #{} '{}' @route '{}' to runnable #{} '{}' input #{}",
                   output_value, self.id(), self.name(), output_route, &destination_id,
                   destination.name(), &io_number);
            run_list.blocked_by(destination_id, self.id());
            run_list.metrics.outputs_sent += 1;
            destination.write_input(io_number, output_value.clone());
            if destination.inputs_satisfied() {
                run_list.inputs_ready(destination_id);
            }
        }
    }
}