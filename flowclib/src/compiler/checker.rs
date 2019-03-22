use model::route::HasRoute;
use generator::generate::GenerationTables;
use model::name::HasName;

/*
    Check that all Functions have connections to all their inputs or return an error
*/
pub fn check_runnable_inputs(tables: &mut GenerationTables) -> Result<(), String> {
    for runnable in &tables.runnables {
        if runnable.get_initial_value().is_none() {
            if let Some(inputs) = runnable.get_inputs() {

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
                            unused_input_count += 1;;
                            error!("Input '{}' at route '{}' of Function '{}' at route '{}' is not used",
                                   input.name(), input.route(), runnable.alias(), runnable.route());
                        }
                    }
                }

                if unused_input_count > 0 {
                    return Err(format!("Function at route '{}' has {} unused inputs",
                                       runnable.route(), unused_input_count));
                }
            }
        }
    }

    Ok(())
}