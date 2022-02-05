#![deny(missing_docs)]
#![feature(proc_macro_span)]
//! `flow_macro` is an attribute macro that inserts code around a `run` function to convert
//! it into a flow `Implementation` with some helper functions for wasm
extern crate proc_macro;

use proc_macro::Span;
use proc_macro::TokenTree::Ident;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use quote::quote;
use syn::DeriveInput;

use flowcore::model::function_definition::FunctionDefinition;

use crate::proc_macro::TokenStream;

#[proc_macro_attribute]
/// Implement the `Flow` macro, an example of which is:
///     #[flow(definition = "definition_file.toml")]
pub fn flow(attr: TokenStream, item: TokenStream) -> TokenStream {

    let definition_filename = find_definition_filename(attr);
//    println!("filename = {}", definition_filename);

    let span = Span::call_site();
    let mut file_path = span.source_file().path().canonicalize().unwrap();

    file_path.set_file_name(definition_filename);
//    println!("path = {}", file_path.display());

    let function_definition = load_function_definition(file_path).unwrap();
//    println!("Function = {:?}", function_definition);

    // Construct a representation of Rust code as a syntax tree that we can manipulate
    let ast = syn::parse(item).unwrap();

    // Build the output token stream with generated code around original supplied code
    generate_code(&ast, &function_definition)
}

// Load a FunctionDefinition from the file at `path`
fn load_function_definition(path: PathBuf) -> Result<FunctionDefinition, String> {
    let mut f = File::open(path).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    toml::from_slice(&buffer).map_err(|e| e.to_string())
}

// Parse the attributes of the macro invocation (a TokenStream) and find the value assigned
// to the definition 'field'
// TODO there must be a better way to parse this and get the rhv of the expression?
fn find_definition_filename(attributes: TokenStream) -> String {
//    println!("attributes: \"{:?}\"", attributes);
    let mut iter = attributes.into_iter();
    if let Ident(ident) = iter.next().expect("'flow' macro must include ´definition' attribute") {
            match ident.to_string().as_str() {
                "definition" => {
                    let _equals = iter.next().unwrap();
                    let filename = iter.next().unwrap();
                    return filename.to_string().trim_matches('"').to_string();
                }
                attribute => panic!("Unsupported attribute '{}' in 'flow' macro", attribute)
            }
    }

    panic!("'flow' macro must include ´definition' attribute")
}

// Generate the code for the implementation struct, including some extra functions to help
// manage memory and pass parameters to and from wasm from native code
fn generate_code(ast: &DeriveInput, function_definition: &FunctionDefinition) -> TokenStream {
    let name = &ast.ident;
    let docs = &function_definition.docs;

    let gen = quote! {
        use std::os::raw::c_void;
        use flowcore::Implementation;

        #[doc = include_str!(#docs)]
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