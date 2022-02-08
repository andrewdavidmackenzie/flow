#![deny(missing_docs)]
#![feature(proc_macro_span)]
//! `flow_function` is a `proc_macro_attribute` macro that inserts code around a supplied function
//! to form a struct that implements the `Implementation` trait, and adds some helper functions for wasm
extern crate proc_macro;

use proc_macro::{Span, TokenStream};
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{ItemFn, parse_macro_input, ReturnType};

use flowcore::model::function_definition::FunctionDefinition;

#[proc_macro_attribute]
/// Implement the `flow_function` macro, an example usage of which is:
/// `#[flow_function]`
pub fn flow_function(_attr: TokenStream, implementation: proc_macro::TokenStream) -> TokenStream {
    // Get the full path to the file where the macro was used, and join the relative filename from
    // the macro's attributes, to find the path to the function's definition file
    let span = Span::call_site();
    let implementation_file_path = span.source_file().path().canonicalize()
        .expect("the 'flow' macro could not get the full file path of the file where it was invoked");

    let mut definition_file_path = implementation_file_path.clone();
    definition_file_path.set_extension("toml");
//    print!("file_path = {}", file_path.display());

    let function_definition = load_function_definition(&definition_file_path);

    // Build the output token stream with generated code around original supplied code
    generate_code(implementation, implementation_file_path,
                  function_definition, definition_file_path)
}

// Load a FunctionDefinition from the file at `path`
fn load_function_definition(path: &Path) -> FunctionDefinition {
    let mut f = File::open(path)
        .unwrap_or_else(|e| panic!("the 'flow' macro could not open the function definition file '{}'\n{}",
                                      path.display(), e));
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)
        .unwrap_or_else(|e| panic!("the 'flow' macro could not read from the function definition file '{}'\n{}",
                                   path.display(), e));
    toml::from_slice(&buffer)
        .unwrap_or_else(|e| panic!("the 'flow' macro could not deserialize the Toml function definition file
        '{}'\n{}", path.display(), e))
}

// If the function accepts inputs as &[serde_json::Value] then there is no need to extract
// and convert the inputs, otherwise form the expected list of inputs for the implementation
// function from the vector of Values passed in.
fn input_conversion(definition: &FunctionDefinition, definition_file_path: PathBuf,
                    implementation_ast: &ItemFn, implementation_file_path: PathBuf) -> Ident {
    let implementation_name = &implementation_ast.sig.ident;
    let implemented_inputs = &implementation_ast.sig.inputs;

    if implemented_inputs.len() == 1 {
        return format_ident!("inputs");
    }

    if implemented_inputs.len() != definition.inputs.len() {
        panic!("a 'flow_function' macro check failed:\n\
            '{}' define {} inputs ({})\n\
            '{}()' implements {} inputs ({})",
               definition.name, definition.inputs.len(), definition_file_path.display(),
               implementation_name, implemented_inputs.len(), implementation_file_path.display());
    }

//    for input in implemented_inputs {
//        println!(Input name: Input Type);
//    }

    format_ident!("inputs")
}

// check that the return type of the implementation function is what we need. i.e. that it
// matches the Implementation trait's run() method return type
fn check_return_type(return_type: &ReturnType) {
    match return_type {
        ReturnType::Default=> panic!("a 'flow_function' macro check failed:\
                                    implementation's return type is () which does not match the \
                                    Implementation trait run() method return type"),
        ReturnType::Type(_, _return_type) => {}
    }
}

// Generate the code for the implementation struct, including some extra functions to help
// manage memory and pass parameters to and from wasm from native code
fn generate_code(function_implementation: TokenStream,
                 implementation_file_path: PathBuf,
                 definition: FunctionDefinition,
                 definition_file_path: PathBuf
                ) -> TokenStream {
    let implementation: proc_macro2::TokenStream = function_implementation.clone().into();
    let implementation_ast = parse_macro_input!(function_implementation as syn::ItemFn);
    let implementation_name = &implementation_ast.sig.ident;

    check_return_type(&implementation_ast.sig.output);

    let inputs = input_conversion(&definition, definition_file_path,
                         &implementation_ast, implementation_file_path);

    // Generate the code that wraps the provided function, including a copy of the function itself
    let docs_comment = if !definition.docs.is_empty() {
        let docs_file = &definition.docs;
        quote! {
            #[doc = include_str!(#docs_file)]
        }
    } else {
        quote! {
            // No documentation was supplied
        }
    };

    let struct_name = format_ident!("{}", FunctionDefinition::camel_case(&definition.name.to_string()));

    let wasm_boilerplate = quote! {
        use std::os::raw::c_void;

        // Allocate a chunk of memory of `size` bytes in wasm module
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn alloc(size: usize) -> *mut c_void {
            use std::mem;
            let mut buf = Vec::with_capacity(size);
            let ptr = buf.as_mut_ptr();
            mem::forget(buf);
            return ptr as *mut c_void;
        }

        // Wrapper function for running a wasm implementation
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn run_wasm(input_data_ptr: *mut c_void, input_data_length: i32) -> i32 {
            use std::ptr::copy;
            let input_data: Vec<u8> = unsafe {
                Vec::from_raw_parts(input_data_ptr as *mut u8,
                                      input_data_length as usize, input_data_length as usize)
            };

            let inputs: Vec<Value> = serde_json::from_slice(&input_data).unwrap();
            let object = #struct_name {};
            let result = object.run(&inputs);

            let return_data = serde_json::to_vec(&result).unwrap();

            unsafe { copy(return_data.as_ptr(), input_data_ptr as *mut u8, return_data.len()); }

            return_data.len() as i32
        }
    };

    let gen = quote! {
        #[allow(unused_imports)]
        use flowcore::Implementation;
        use flowcore::{RUN_AGAIN, RunAgain};

        #wasm_boilerplate

        #implementation

        #docs_comment
        #[derive(Debug)]
        pub struct #struct_name;

        impl Implementation for #struct_name {
            fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
//                #input_conversion

                #implementation_name(#inputs)
            }
        }

    };
    gen.into()
}

// Parse the attributes of the macro invocation (a TokenStream) and find the value assigned
// to the definition 'field'
// TODO there must be a better way to parse this and get the rhv of the expression?
// If we go back to specifying the filename
// #[flow_function(definition = "definition_file.toml")]
// then we can use this code
// use proc_macro::TokenTree::Ident;
//    let definition_filename = find_definition_filename(attr);
// definition_file_path.set_file_name(definition_filename);
/*
fn find_definition_filename(attributes: TokenStream) -> String {
    let mut iter = attributes.into_iter();
    if let Ident(ident) = iter.next().expect("the 'flow' macro must include ´definition' attribute") {
            match ident.to_string().as_str() {
                "definition" => {
                    let _equals = iter.next().expect("the 'flow' macro expect '=' after 'definition' attribute");
                    let filename = iter.next()
                        .expect("the 'flow' macro expected name of definition TOML file after '=' in 'definition' attribute");
                    return filename.to_string().trim_matches('"').to_string();
                }
                attribute => panic!("the 'flow' macro does not support the '{}' attribute", attribute)
            }
    }

    panic!("the 'flow' macro must include the ´definition' attribute")
}
 */