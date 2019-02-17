use model::route::Route;
use model::io::IOSet;
use model::name::HasName;
use model::route::HasRoute;
use serde_json::Value as JsonValue;
use std::fmt;

pub trait Runnable: fmt::Display + HasName + HasRoute {
    fn set_id(&mut self, id: usize);
    fn get_id(&self) -> usize;
    fn get_inputs(&self) -> IOSet;
    fn get_outputs(&self) -> IOSet;
    fn add_output_route(&mut self, output_route: (Route, usize, usize)); // Route is the output subroute
    fn get_source_url(&self) -> Option<String>;
    fn get_type(&self) -> &str;
    fn is_static_value(&self) -> bool;
    fn get_output_routes(&self) -> &Vec<(Route, usize, usize)>;
    fn get_initial_value(&self) -> Option<JsonValue>;
    fn get_implementation_source(&self) -> String;
}