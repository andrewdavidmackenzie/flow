use error_chain::bail;

use flowcore::input::InputInitializer::Always;

use crate::errors::*;
use crate::generator::generate::GenerationTables;
use crate::model::route::HasRoute;
use crate::model::route::Route;

/*
    Check for a series of potential problems in connections
*/
pub fn check_connections(tables: &mut GenerationTables) -> Result<()> {
    check_for_competing_inputs(tables)?;

    Ok(())
}

/*
    Check for a problems that lead to competition for inputs causing input overflow:
    - A single function has two output connections to the same destination input
    - a function connects to an input that has a constant initializer
*/
fn check_for_competing_inputs(tables: &GenerationTables) -> Result<()> {
    for connection in &tables.collapsed_connections {
        // check for ConstantInitializer at destination
        if let Some(Always(_)) = connection.to_io().get_initializer() {
            bail!(
                "Connection from '{}' to input at '{}' that also has a `always` initializer",
                connection.from_io().route(),
                connection.to_io().route()
            );
        }
    }

    Ok(())
}

/// Check that all Functions have connections to all their inputs or return an error
pub fn check_function_inputs(tables: &mut GenerationTables) -> Result<()> {
    for function in &tables.functions {
        for input in function.get_inputs() {
            match input.get_initializer() {
                None => {
                    if !connection_to(tables, input.route()) {
                        bail!("Input at route '{}' is not used", input.route());
                    }
                }
                Some(Always(_)) => {
                    // Has a constant initializer and there is another
                    // connections to this input then flag that as an error
                    if connection_to(tables, input.route()) {
                        bail!("Input at route '{}' has a 'constant' initializer and a connection to it",
                                               input.route());
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Check that some impure function producing a side effect is called or return an error
pub fn check_side_effects(tables: &mut GenerationTables) -> Result<()> {
    for function in &tables.functions {
        // Until we separate impure inputs and side-effects we will assume that if a function
        // is impure and has inputs then it has side-effects
        if function.is_impure() && !function.inputs.is_empty() {
            return Ok(());
        }
    }

    bail!("Flow has no side-effects")
}

fn connection_to(tables: &GenerationTables, input: &Route) -> bool {
    for connection in &tables.collapsed_connections {
        if connection.to_io().route() == input {
            return true;
        }
    }
    false
}
