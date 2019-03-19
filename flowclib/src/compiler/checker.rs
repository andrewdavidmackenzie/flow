use model::route::HasRoute;
use generator::generate::GenerationTables;

/*
    Check that all processes have connections to all their inputs or return an error
    - with execption of a value that has an initial value
*/
pub fn check_process_inputs(tables: &mut GenerationTables) -> Result<(), String> {
    for runnable in &tables.runnables {
        if runnable.get_initial_value().is_none() {
            if let Some(inputs) = runnable.get_inputs() {
                for input in inputs {
                    let mut found = input.get_initial_value().is_some();

                    for connection in &tables.collapsed_connections {
                        if connection.to_io.route() == input.route() {
                            found = true;
                        }
                    }
                    if !found {
                        return Err(format!("Could not find any connection to process '{}' input with route '{}', so it can never run.",
                                           runnable.route(), input.route()));
                    }
                }
            }
        }
    }

    Ok(())
}