use model::route::HasRoute;
use generator::generate::GenerationTables;
use model::name::HasName;

/*
    Check that all Functions have connections to all their inputs or return an error
*/
pub fn check_function_inputs(tables: &mut GenerationTables) -> Result<(), String> {
    for function in &tables.functions {
        if let Some(inputs) = function.get_inputs() {
            let mut unused_input_count = 0;
            for input in inputs {
                if input.get_initial_value().is_none() {
                    let mut found = false;
                    for connection in &tables.collapsed_connections {
                        if connection.to_io.route() == input.route() {
                            found = true;
                        }
                    }
                    if !found {
                        unused_input_count += 1;
                        ;
                        error!("Input '{}' at route '{}' of Function '{}' at route '{}' is not used",
                               input.name(), input.route(), function.alias(), function.route());
                    }
                }
            }

            if unused_input_count > 0 {
                return Err(format!("Function at route '{}' has {} unused inputs",
                                   function.route(), unused_input_count));
            }
        }
    }

    Ok(())
}