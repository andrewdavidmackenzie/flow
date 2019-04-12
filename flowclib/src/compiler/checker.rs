use model::route::HasRoute;
use generator::generate::GenerationTables;
use model::name::HasName;
use model::route::Route;
use flowrlib::input::InputInitializer::Constant;

/*
    Check that all Functions have connections to all their inputs or return an error
*/
pub fn check_function_inputs(tables: &mut GenerationTables) -> Result<(), String> {
    for function in &tables.functions {
        if let Some(inputs) = function.get_inputs() {
            for input in inputs {
                match input.get_initializer() {
                    None => {
                        if !connection_to(tables, &input.route()) {
                            return Err(format!("Input '{}' at route '{}' of Function '{}' at route '{}' is not used",
                                               input.name(), input.route(), function.alias(), function.route()));
                        }
                    }
                    Some(Constant(_)) => {
                        // Has a constant initializer and there is another
                        // connections to this input then flag that as an error
                        if connection_to(tables, &input.route()) {
                            return Err(format!("Input '{}' at route '{}' of Function '{}' at route '{}' has a 'constant' initializer and a connection to it",
                                               input.name(), input.route(), function.alias(), function.route()));
                        }
                    },
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn connection_to(tables: &GenerationTables, input: &Route) -> bool {
    for connection in &tables.collapsed_connections {
        if connection.to_io.route() == input {
            return true;
        }
    }
    false
}