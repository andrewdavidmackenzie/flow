use error_chain::bail;
use log::info;

use flowcore::model::input::InputInitializer::Always;
use flowcore::model::io::IO;
use flowcore::model::route::HasRoute;

use crate::compiler::compile::CompilerTables;
use crate::errors::*;

/// Check that all Functions have connections to all their inputs or return an error
/// All inputs must be connected and receive values at run-time or a function can never run
/// This is different from Outputs can be used selectively, and so if one is not connected that
/// is not a problem for compiling or running necessarily.
pub fn check_function_inputs(tables: &mut CompilerTables) -> Result<()> {
    info!("\n=== Compiler: Checking all Function Inputs are connected");
    for function in &tables.functions {
        for input in function.get_inputs() {
            if input.get_initializer().is_none() && input.get_flow_initializer().is_none()
                && tables.connection_to(input.route()).is_none() {
                bail!("Input at route '{}' is not connected to nor initialized", input.route());
            }

            // If has a Constant initializer and a connections then flag that as an error
            if got_constant_initializer(input) && tables.connection_to(input.route()).is_some() {
                bail!("Input at route '{}' has a 'Constant' initializer and a connection to it",
                                       input.route());
            }
        }
    }

    info!("No problems found. All functions have connections to all their inputs");
    Ok(())
}

fn got_constant_initializer(input: &IO) -> bool {
    matches!(input.get_initializer(), Some(Always(_)))
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
