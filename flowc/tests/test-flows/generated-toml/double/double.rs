/// Double a number
use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[flow_function]
fn inner_double(value: f64) -> Result<(Option<Value>, RunAgain)> {
    flow_output!(json!(value * 2.0))
}
