use model::route::HasRoute;
use generator::generate::GenerationTables;

/*
    Check that all processes have connections to all their inputs or return an error
*/
pub fn check_process_inputs(tables: &mut GenerationTables) -> Result<(), String> {
    for runnable in &tables.runnables {
        if runnable.get_initial_value().is_none() {
            if let Some(inputs) = runnable.get_inputs() {

                let mut connected_input_count = 0;
                for input in inputs {
                    if input.get_initial_value().is_some() {
                        connected_input_count += 1;
                    } else {
                        let mut found = false;
                        for connection in &tables.collapsed_connections {
                            if connection.to_io.route() == input.route() {
                                found = true;
                            }
                        }
                        if found {
                            connected_input_count += 1;;
                        }
                    }
                }

                if connected_input_count != inputs.len() {
                    return Err(format!("Process at route '{}' has at least one unused input",
                                       runnable.route()));
                }
            }
        }
    }

    Ok(())
}