use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use log::info;
use url::Url;

use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::flow_manifest::{DEFAULT_MANIFEST_FILENAME, FlowManifest};
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::input::Input;
use flowcore::model::metadata::MetaData;
use flowcore::model::name::HasName;
#[cfg(feature = "debugger")]
use flowcore::model::route::HasRoute;
use flowcore::model::runtime_function::RuntimeFunction;

use crate::compiler::tables::CompilerTables;
use crate::errors::*;

/// Paths in the manifest are relative to the location of the manifest file, to make the file
/// and associated files relocatable (and maybe packaged into a ZIP etc). So we use manifest_url
/// as the location other file paths are made relative to.
pub fn create_manifest(
    flow: &FlowDefinition,
    debug_symbols: bool,
    manifest_url: &Url,
    tables: &CompilerTables,
    #[cfg(feature = "debugger")] source_urls: HashSet<(Url, Url)>,
) -> Result<FlowManifest> {
    info!("Writing flow manifest to '{}'", manifest_url);

    let mut manifest = FlowManifest::new(MetaData::from(flow));

    // Generate run-time Function struct for each of the compile-time functions
    for function in &tables.functions {
        manifest.add_function(function_to_runtimefunction(
            manifest_url,
            function,
            debug_symbols,
        ).chain_err(|| "Could not convert function to runtime function")?);
    }

    manifest.set_lib_references(&tables.libs);
    manifest.set_context_references(&tables.context_functions);
    #[cfg(feature = "debugger")]
    manifest.set_source_urls(source_urls);

    Ok(manifest)
}

/// Generate a manifest for the flow in JSON that can be used to run it using 'flowr'
// TODO this is tied to being a file:// - generalize this to write to a URL, moving the code
// TODO into the provider and implementing for file and http
pub fn write_flow_manifest(
    flow: FlowDefinition,
    debug_symbols: bool,
    destination: &Path,
    tables: &CompilerTables,
    #[cfg(feature = "debugger")] source_urls: HashSet<(Url, Url)>,
) -> Result<PathBuf> {
    let mut filename = destination.to_path_buf();
    filename.push(DEFAULT_MANIFEST_FILENAME);
    filename.set_extension("json");
    let mut manifest_file =
        File::create(&filename).chain_err(|| "Could not create manifest file")?;
    let manifest_url =
        Url::from_file_path(&filename).map_err(|_| "Could not parse Url from file path")?;
    let manifest = create_manifest(
        &flow,
        debug_symbols,
        &manifest_url,
        tables,
        #[cfg(feature = "debugger")]
        source_urls,
    )
    .chain_err(|| "Could not create manifest from parsed flow and compiler tables")?;

    manifest_file
        .write_all(
            serde_json::to_string_pretty(&manifest)
                .chain_err(|| "Could not pretty format the manifest JSON contents")?
                .as_bytes(),
        )
        .chain_err(|| "Could not write manifest data bytes to created manifest file")?;

    Ok(filename)
}

/*
    Create a run-time function struct from a compile-time function struct.
    manifest_dir is the directory that paths will be made relative to.
*/
fn function_to_runtimefunction(
    manifest_url: &Url,
    function: &FunctionDefinition,
    debug_symbols: bool,
) -> Result<RuntimeFunction> {
    #[cfg(feature = "debugger")]
    let name = if debug_symbols {
        function.alias().to_string()
    } else {
        "".to_string()
    };

    #[cfg(feature = "debugger")]
    let route = if debug_symbols {
        function.route().to_string()
    } else {
        "".to_string()
    };

    // make the location of implementation relative to the output directory if it is under it
    let implementation_location = implementation_location_relative(function, manifest_url)
        .chain_err(|| "Could not create Url for relative implementation location")?;

    let mut runtime_inputs = vec![];
    for input in function.get_inputs() {
        runtime_inputs.push(Input::from(input));
    }

    Ok(RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        name,
        #[cfg(feature = "debugger")]
        route,
        implementation_location,
        runtime_inputs,
        function.get_id(),
        function.get_flow_id(),
        function.get_output_connections(),
        debug_symbols,
    ))
}

/*
    Get the location of the implementation - relative to the Manifest if it is a provided implementation
*/
fn implementation_location_relative(function: &FunctionDefinition, manifest_url: &Url) -> Result<String> {
    if let Some(ref lib_reference) = function.get_lib_reference() {
        Ok(lib_reference.to_string())
    } else if let Some(ref context_reference) = function.get_context_reference() {
        Ok(context_reference.to_string())
    } else {
        let implementation_path = function.get_implementation();
        let implementation_url = Url::from_file_path(implementation_path)
            .map_err(|_| {
                format!(
                    "Could not create Url from file path: {}",
                    implementation_path
                )
            })?
            .to_string();

        let mut manifest_base_url = manifest_url.clone();
        manifest_base_url
            .path_segments_mut()
            .map_err(|_| "cannot be base")?
            .pop();

        info!("Manifest base = '{}'", manifest_base_url.to_string());
        info!("Absolute implementation path = '{}'", implementation_path);
        let relative_path =
            implementation_url.replace(&format!("{}/", manifest_base_url.as_str()), "");
        info!("Relative implementation path = '{}'", relative_path);
        Ok(relative_path)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use url::Url;

    use flowcore::model::datatype::{ARRAY_TYPE, OBJECT_TYPE, STRING_TYPE};
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::input::InputInitializer;
    use flowcore::model::io::IO;
    use flowcore::model::name::Name;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::output_connection::Source::Output;
    use flowcore::model::route::Route;

    use super::function_to_runtimefunction;

    #[test]
    fn function_with_sub_route_output_generation() {
        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![
                IO::new(vec!(OBJECT_TYPE.into()), Route::default()),
                IO::new(vec!(STRING_TYPE.into()), Route::default()),
            ],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            None,
            Some(Url::parse("context://stdio/stdout").expect("Could not parse Url")),
            vec![
                OutputConnection::new(
                    Source::default(),
                    1,
                    0,
                    0,
                    0,
                    false,
                    String::default(),
                    #[cfg(feature = "debugger")]
                    String::default(),
                    0,
                ),
                OutputConnection::new(
                    Output("sub_route".into()),
                    2,
                    0,
                    0,
                    0,
                    false,
                    String::default(),
                    #[cfg(feature = "debugger")]
                    String::default(),
                    0,
                ),
            ],
            0,
            0,
        );

        let expected = "{
  'function_id': 0,
  'flow_id': 0,
  'implementation_location': 'context://stdio/stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0,
      'priority': 0
    },
    {
      'source': {
        'Output': 'sub_route'
      },
      'function_id': 2,
      'io_number': 0,
      'flow_id': 0,
      'priority': 0
    }
  ]
}";

        let br = Box::new(function) as Box<FunctionDefinition>;

        let runtime_process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&runtime_process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace('\'', "\""));
    }

    #[test]
    fn function_generation() {
        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new(vec!(STRING_TYPE.into()), Route::default())],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            None,
            Some(Url::parse("context://stdio/stdout").expect("Could not parse Url")),
            vec![OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            )],
            0,
            0,
        );

        let expected = "{
  'function_id': 0,
  'flow_id': 0,
  'implementation_location': 'context://stdio/stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0,
      'priority': 0
    }
  ]
}";

        let br = Box::new(function) as Box<FunctionDefinition>;

        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace('\'', "\""));
    }

    #[test]
    fn function_generation_with_array_order() {
        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new(vec!(STRING_TYPE.into()), Route::default())],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            None,
            Some(Url::parse("context://stdio/stdout").expect("Could not parse Url")),
            vec![OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                1,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            )],
            0,
            0,
        );

        let expected = "{
  'function_id': 0,
  'flow_id': 0,
  'implementation_location': 'context://stdio/stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0,
      'destination_array_order': 1,
      'priority': 0
    }
  ]
}";

        let br = Box::new(function) as Box<FunctionDefinition>;

        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace('\'', "\""));
    }

    #[test]
    fn function_with_initialized_input_generation() {
        let mut io = IO::new(vec!(STRING_TYPE.into()), Route::default());
        io.set_initializer(&Some(InputInitializer::Once(json!(1))));

        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout".to_string(),
            Name::from("print"),
            vec![io],
            vec![],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            None,
            Some(Url::parse("context://stdio/stdout").expect("Could not parse Url")),
            vec![],
            0,
            0,
        );

        let expected = "{
  'function_id': 0,
  'flow_id': 0,
  'implementation_location': 'context://stdio/stdout',
  'inputs': [
    {
      'initializer': {
        'once': 1
      }
    }
  ]
}";

        let br = Box::new(function) as Box<FunctionDefinition>;
        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(expected.replace('\'', "\""), serialized_process);
    }

    #[test]
    fn function_with_constant_input_generation() {
        let mut io = IO::new(vec!(STRING_TYPE.into()), Route::default());
        io.set_initializer(&Some(InputInitializer::Always(json!(1))));

        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout".to_string(),
            Name::from("print"),
            vec![io],
            vec![],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            None,
            Some(Url::parse("context://stdio/stdout").expect("Could not parse Url")),
            vec![],
            0,
            0,
        );

        let expected = "{
  'function_id': 0,
  'flow_id': 0,
  'implementation_location': 'context://stdio/stdout',
  'inputs': [
    {
      'initializer': {
        'always': 1
      }
    }
  ]
}";

        let br = Box::new(function) as Box<FunctionDefinition>;
        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(expected.replace('\'', "\""), serialized_process);
    }

    #[test]
    fn function_with_array_input_generation() {
        let io = IO::new(vec!("array/string".into()), Route::default());

        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout".to_string(),
            Name::from("print"),
            vec![io],
            vec![],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            None,
            Some(Url::parse("context://stdio/stdout").expect("Could not parse Url")),
            vec![],
            0,
            0,
        );

        let expected = "{
  'function_id': 0,
  'flow_id': 0,
  'implementation_location': 'context://stdio/stdout',
  'inputs': [
    {}
  ]
}";

        let br = Box::new(function) as Box<FunctionDefinition>;
        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace('\'', "\""));
    }

    fn test_function() -> FunctionDefinition {
        FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new(vec!(STRING_TYPE.into()), Route::default())],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            None,
            Some(Url::parse("context://stdio/stdout")
                .expect("Could not parse Url")),
            vec![OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            )],
            0,
            0,
        )
    }

    #[test]
    fn function_to_code_with_debug_generation() {
        let function = test_function();

        #[cfg(feature = "debugger")]
        let expected = "{
  'name': 'print',
  'route': '/flow0/stdout',
  'function_id': 0,
  'flow_id': 0,
  'implementation_location': 'context://stdio/stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0,
      'priority': 0
    }
  ]
}";
        #[cfg(not(feature = "debugger"))]
        let expected = "{
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'context://stdio/stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0,
      'priority': 0
    }
  ]
}";
        let br = Box::new(function) as Box<FunctionDefinition>;

        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            true,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace('\'', "\""));
    }

    #[test]
    fn function_with_array_element_output_generation() {
        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new(vec!(ARRAY_TYPE.into()), Route::default())],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            None,
            Some(Url::parse("context://stdio/stdout").expect("Could not parse Url")),
            vec![OutputConnection::new(
                Output("/0".into()),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            )],
            0,
            0,
        );

        let expected = "{
  'function_id': 0,
  'flow_id': 0,
  'implementation_location': 'context://stdio/stdout',
  'output_connections': [
    {
      'source': {
        'Output': '/0'
      },
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0,
      'priority': 0
    }
  ]
}";

        let br = Box::new(function) as Box<FunctionDefinition>;

        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace('\'', "\""));
    }
}
