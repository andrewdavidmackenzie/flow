#![deny(missing_docs)]
//! `flow_macro` is an attribute macro that inserts code around a `run` function to convert
//! it into a flow `Implementation` with some helper functions for wasm
extern crate proc_macro;

use proc_macro::TokenTree::Ident;

use quote::quote;

use crate::proc_macro::TokenStream;

/*
use simpath::Simpath;
use std::path::PathBuf;
use url::Url;
use flowcore::deserializers::deserializer::get_deserializer;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::process::Process;
use flowcore::model::process::Process::FunctionProcess;
use flowcore::lib_provider::{MetaProvider, Provider};
*/
#[proc_macro_attribute]
/// Implement the `Flow` macro, an example of which is:
///     #[flow(definition = "definition_file.toml")]
pub fn flow(attr: TokenStream, item: TokenStream) -> TokenStream {
    let definition_filename = find_definition_filename(attr).unwrap();
    println!("filename = {}", definition_filename);

//    let _function_definition = load_function_definition(definition_filename).unwrap();

    // Construct a representation of Rust code as a syntax tree that we can manipulate
    let ast = syn::parse(item).unwrap();

    // Build the output token stream with generated code around original supplied code
    generate_code(&ast)
}

// Load a FunctionDefinition from the specified filename relative to this file
/*fn load_function_definition(filename: String) -> Option<FunctionDefinition> {
    let url = Url::from_file_path(PathBuf::from(filename)).unwrap();
    println!("url = {}", url);
    let provider = MetaProvider::new(Simpath::new(""));

    let content = provider.get_contents(&url).unwrap();
    let content_string = String::from_utf8(content).unwrap();

    let deserializer = get_deserializer::<Process>(&url).unwrap();
    let process = deserializer.deserialize(&content_string, Some(&url)).unwrap();

    match process {
        FunctionProcess(function) => Some(function),
        _ => None
    }
}
*/
// Parse the attributes of the macro invocation (a TokenStream) and find the value assigned
// to the definition 'field'
fn find_definition_filename(attributes: TokenStream) -> Option<String> {
//    println!("attributes: \"{:?}\"", attributes);
    let mut iter = attributes.into_iter();
    while let Some(attribute) = iter.next() {
        if let Ident(ident) = &attribute {
            if ident.to_string() == "definition" {
                let _p = iter.next().unwrap();
                let f = iter.next().unwrap();
                return Some(f.to_string());
            }
        }
    }

    None
}

// Generate the code for the implementation struct, including some extra functions to help
// manage memory and pass parameters to and from wasm from native code
fn generate_code(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        use std::os::raw::c_void;

        /// This is the struct that will carry the implementation
        #[derive(Debug)]
        pub struct #name;

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
            let object = #name {};
            let result = object.run(&inputs);

            let return_data = serde_json::to_vec(&result).unwrap();

            unsafe { copy(return_data.as_ptr(), input_data_ptr as *mut u8, return_data.len()); }

            return_data.len() as i32
        }
    };
    gen.into()
}