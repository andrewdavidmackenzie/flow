#![feature(proc_macro_span)]
#![allow(unexpected_cfgs)]
//! `flow_function` is a `proc_macro_attribute` macro that wraps a `fn` with a struct and a method
//! to implement the [Implementation][flowcore::Implementation] trait, so it can be used as the
//! implementation of a flow function.

extern crate proc_macro;

use proc_macro::{Span, TokenStream};
use std::fs;
use std::path::Path;

use proc_macro2::Ident;
use quote::{format_ident, quote, ToTokens};
use syn::{ItemFn, parse_macro_input, ReturnType};

use flowcore::model::function_definition::FunctionDefinition;

/// The `flow_function` macro definition
///
/// # Panics
///
/// Will panic if the file path of the source file where the macro was used cannot be determined.
#[proc_macro_attribute]
pub fn flow_function(_attr: TokenStream, implementation: TokenStream) -> TokenStream {
    // Get the full path to the file where the macro was used, and join the relative filename from
    // the macro's attributes, to find the path to the function's definition file
    let mut definition_file_path = Span::call_site().local_file()
        .expect("the 'flow' macro could not get the file path where macro was invoked");
    definition_file_path.set_extension("toml");

    let function_definition = load_function_definition(&definition_file_path);

    // Build the output token stream with generated code around original supplied code
    generate_code(implementation, &function_definition)
}

// Load a `FunctionDefinition` from the file at `path`
fn load_function_definition(path: &Path) -> FunctionDefinition {
    let function = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("'flow' macro could not read from the function definition file '{}'\n{e}",
                                   path.display()));
    toml::from_str(&function)
        .unwrap_or_else(|e| panic!("'flow' macro could not deserialize the Toml function definition file
        '{}'\n{e}", path.display()))
}

// If the function accepts inputs as &[serde_json::Value] then there is no need to extract
// and convert the inputs, otherwise form the expected list of inputs for the implementation
// function from the vector of Values passed in.
// Full of hacks as TokenStream2 from into_token_stream() doesn't implement PartialEq to be
// able to compare it with a quote!() version of what I'm expecting
fn input_conversion(definition: &FunctionDefinition, implementation_ast: &ItemFn) -> Ident {
    let implementation_name = &implementation_ast.sig.ident;
    let implemented_inputs = &implementation_ast.sig.inputs;

    // if there is only one input (`inputs`) and it matches the expected standard form (`&[Value]`)
    if implemented_inputs.len() == 1 {
        let input = implemented_inputs.first()
            .expect("the 'flow' macro could not get the function's first argument type");

        if input.into_token_stream().to_string() == quote! { inputs : &[Value] }.to_string() {
            return format_ident!("inputs");
        }
    }

    // perform some checks before attempting input conversion
    assert_eq!(implemented_inputs.len(), definition.inputs.len(), "a 'flow_function' macro check failed:\n\
            '{}' define {} inputs\n\
            '{}()' implements {} inputs",
               definition.name, definition.inputs.len(),
               implementation_name, implemented_inputs.len());

    // TODO If function accepts types directly (not `&[Value]`), check they match function definition
    // for input_pair in implemented_inputs.pairs() {
    //    match input_pair {
    //       Punctuated(t, p) => {
    //           println!("FnArg: {:?}", t);
    //           match t {
    //               Receiver(r) => println!("error, self not allowed"),
    //               Typed(pt) => {
    //                  println!("PatType: {:?}", pt));
    //                  println!("Input name: {}", pt.pat);
    //                  println!("Input type: {:?}", pt.ty);
    //               }
    //           }
    //       End(_) = {}
    //       }
    //    }
    // }

    unimplemented!()
}

// check that the return type of the implementation function is what we need. i.e. that it
// matches the Implementation trait's run() method return type
// Hacky but works for now - find a better way to do it
fn check_return_type(return_type: &ReturnType) {
    assert_eq!(return_type.into_token_stream().to_string(),
               quote! { -> Result<(Option<Value>, RunAgain)>}.to_string(),
                "a 'flow_function' macro check failed:\n\
                                    implementation's return type does not match the \
                                    Implementation trait's run() method return type");
}

// Generate the code for the implementation struct, including some extra functions to help
// manage memory and pass parameters to and from a wasm compiled version of it
fn generate_code(function_implementation: TokenStream,
                 definition: &FunctionDefinition) -> TokenStream {
    let implementation: proc_macro2::TokenStream = function_implementation.clone().into();
    let implementation_ast = parse_macro_input!(function_implementation as syn::ItemFn);
    let implementation_name = &implementation_ast.sig.ident;

    check_return_type(&implementation_ast.sig.output);

    let input_list = input_conversion(definition, &implementation_ast);

    let number_of_defined_inputs = definition.inputs.len();

    // generate code that does a runtime check on the number of values in the 'inputs' array
    // matches the number of inputs in the FunctionDefinition
    let input_number_check = quote! {
        // check at run time that the number of values in inputs matches the inputs number expected
        if inputs.len() != #number_of_defined_inputs {
            flowcore::errors::bail!("'inputs' does not have the expected number of input values");
        }
    };

    // Generate the code that wraps the provided function, including a copy of the function itself
    let docs_comment = if definition.docs.is_empty() {
        quote! {
            // No documentation was supplied
        }
    } else {
        let docs_file = &definition.docs;
        quote! {
            #[doc = include_str!(#docs_file)]
        }
    };

    let struct_name = format_ident!("{}", FunctionDefinition::camel_case(&definition.name.to_string()));

    // This code will be compiled to wasm along with the Implementation's run() function
    // and it will be running on the wasm side - hence it includes code to build the serde_json
    // input structure expected by run(), and build a flat memory return from the serde_json
    // returned from run()
    let wasm_boilerplate = quote! {
        // Allocate a chunk of memory of `size` bytes in wasm module
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn alloc(size: usize) -> *mut std::os::raw::c_void {
            use std::mem;
            let mut buf = Vec::with_capacity(size);
            let ptr = buf.as_mut_ptr();
            mem::forget(buf);
            return ptr as *mut std::os::raw::c_void;
        }

        // Wrapper function for running a wasm implementation
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn run_wasm(input_data_ptr: *mut std::os::raw::c_void, input_data_length: i32) -> i32 {
            use std::ptr::copy;
            let input_data: &[u8] = unsafe {
                std::slice::from_raw_parts(input_data_ptr as *mut u8,
                                      input_data_length as usize)
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
        #wasm_boilerplate

        #implementation

        #docs_comment
        #[derive(Debug)]
        pub struct #struct_name;
        use flowcore::Implementation;
        impl Implementation for #struct_name {
            fn run(&self, inputs: &[Value]) -> flowcore::errors::Result<(Option<Value>, flowcore::RunAgain)> {
//                #input_conversion
                #input_number_check

                #implementation_name(#input_list)
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