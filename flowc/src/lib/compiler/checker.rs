use error_chain::bail;
use log::info;

use flowcore::model::input::InputInitializer::Always;
use flowcore::model::route::HasRoute;
use flowcore::model::route::Route;

use crate::compiler::compile::CompilerTables;
use crate::errors::*;

/// Check for a series of potential problems in connections
pub fn check_connections(tables: &mut CompilerTables) -> Result<()> {
    info!("\n=== Compiler: Checking connections");
    check_for_competing_inputs(tables)?;
    info!("No problems found in connections");
    Ok(())
}

/*
    Check for a problems that lead to competition for inputs causing input overflow:
    - a function connects to an input that has a constant initializer
*/
fn check_for_competing_inputs(tables: &CompilerTables) -> Result<()> {
    for connection in &tables.collapsed_connections {
        // check for `Always` type of Initializer at destination
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
/// All inputs must be connected and receive values at run-time or a function can never run
/// This is different from Outputs can be used selectively, and so if one is not connected that
/// is not a problem for compiling or running necessarily.
pub fn check_function_inputs(tables: &mut CompilerTables) -> Result<()> {
    info!("\n=== Compiler: Checking all Function Inputs are connected");
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

    info!("No problems found. All functions have connections to all their inputs");
    Ok(())
}

/// Check that some impure function producing a side effect is called or return an error
pub fn check_side_effects(tables: &mut CompilerTables) -> Result<()> {
    info!("\n=== Compiler: Checking flow has side-effects");
    for function in &tables.functions {
        // Until we separate impure inputs and side-effects we will assume that if a function
        // is impure and has inputs then it has side-effects
        if function.is_impure() && !function.inputs.is_empty() {
            info!("Flow has side effects from 1 or more functions");
            return Ok(());
        }
    }

    bail!("Flow has no side-effects")
}

fn connection_to(tables: &CompilerTables, input: &Route) -> bool {
    for connection in &tables.collapsed_connections {
        if connection.to_io().route() == input {
            return true;
        }
    }
    false
}
