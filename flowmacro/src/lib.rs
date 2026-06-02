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
use syn::{parse_macro_input, FnArg, ItemFn, Pat, ReturnType, Type};

use flowcore::model::function_definition::FunctionDefinition;

/// The `flow_function` macro definition
///
/// # Panics
///
/// Will panic if the file path of the source file where the macro was used cannot be determined.
#[proc_macro_attribute]
pub fn flow_function(_attr: TokenStream, implementation: TokenStream) -> TokenStream {
    let mut definition_file_path = Span::call_site()
        .local_file()
        .expect("the 'flow' macro could not get the file path where macro was invoked");
    definition_file_path.set_extension("toml");

    let function_definition = load_function_definition(&definition_file_path);

    generate_code(implementation, &function_definition)
}

fn load_function_definition(path: &Path) -> FunctionDefinition {
    let function = fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "'flow' macro could not read from the function definition file '{}'\n{e}",
            path.display()
        )
    });
    toml::from_str(&function).unwrap_or_else(|e| {
        panic!(
            "'flow' macro could not deserialize the Toml function definition file\n\
        '{}'\n{e}",
            path.display()
        )
    })
}

/// Determines how to call the implementation function based on its signature.
///
/// If the function takes `inputs: &[Value]` — pass the slice directly (legacy mode).
/// If the function takes typed parameters — generate extraction code and pass extracted values.
enum InputMode {
    /// Pass the raw `&[Value]` slice directly
    SliceMode(Ident),
    /// Extract typed values from the slice and pass them as arguments
    TypedMode {
        extraction: proc_macro2::TokenStream,
        call_args: proc_macro2::TokenStream,
    },
}

fn analyze_inputs(definition: &FunctionDefinition, implementation_ast: &ItemFn) -> InputMode {
    let implementation_name = &implementation_ast.sig.ident;
    let implemented_inputs = &implementation_ast.sig.inputs;

    // Legacy mode: single parameter `inputs: &[Value]`
    if implemented_inputs.len() == 1 {
        let input = implemented_inputs
            .first()
            .expect("the 'flow' macro could not get the function's first argument type");

        if input.into_token_stream().to_string() == quote! { inputs : &[Value] }.to_string() {
            return InputMode::SliceMode(format_ident!("inputs"));
        }
    }

    // Typed mode: parameters match the function definition's inputs
    assert_eq!(
        implemented_inputs.len(),
        definition.inputs.len(),
        "a 'flow_function' macro check failed:\n\
            '{}' defines {} inputs\n\
            '{}()' implements {} inputs",
        definition.name,
        definition.inputs.len(),
        implementation_name,
        implemented_inputs.len()
    );

    let mut extractions = Vec::new();
    let mut call_args = Vec::new();

    for (i, fn_arg) in implemented_inputs.iter().enumerate() {
        if let FnArg::Typed(pat_type) = fn_arg {
            let param_name = match pat_type.pat.as_ref() {
                Pat::Ident(pat_ident) => &pat_ident.ident,
                _ => panic!("flow_function: unsupported parameter pattern"),
            };

            let extraction = generate_extraction(i, param_name, &pat_type.ty);
            extractions.push(extraction);
            call_args.push(quote! { #param_name });
        } else {
            panic!("flow_function: 'self' parameters are not supported");
        }
    }

    let extraction = quote! { #(#extractions)* };
    let call_args = quote! { #(#call_args),* };

    InputMode::TypedMode {
        extraction,
        call_args,
    }
}

/// Generate code to extract a single input value from `inputs[index]` into a typed variable.
fn generate_extraction(index: usize, name: &Ident, ty: &Type) -> proc_macro2::TokenStream {
    let type_str = ty.into_token_stream().to_string();
    let name_str = name.to_string();

    match type_str.as_str() {
        // Reference to Value — just index into the slice
        "& Value" | "& serde_json :: Value" => {
            quote! {
                let #name: &serde_json::Value = inputs.get(#index)
                    .ok_or(concat!("Could not get input '", #name_str, "'"))?;
            }
        }
        // Owned Value
        "Value" | "serde_json :: Value" => {
            quote! {
                let #name: serde_json::Value = inputs.get(#index)
                    .ok_or(concat!("Could not get input '", #name_str, "'"))?.clone();
            }
        }
        // Reference to Number — extract and validate
        "& Number" | "& serde_json :: Number" => {
            quote! {
                let #name: &serde_json::Number = inputs.get(#index)
                    .ok_or(concat!("Could not get input '", #name_str, "'"))?
                    .as_number()
                    .ok_or(concat!("Input '", #name_str, "' is not a number"))?;
            }
        }
        // f64
        "f64" => {
            quote! {
                let #name: f64 = inputs.get(#index)
                    .ok_or(concat!("Could not get input '", #name_str, "'"))?
                    .as_f64()
                    .ok_or(concat!("Input '", #name_str, "' is not a number"))?;
            }
        }
        // i64
        "i64" => {
            quote! {
                let #name: i64 = inputs.get(#index)
                    .ok_or(concat!("Could not get input '", #name_str, "'"))?
                    .as_i64()
                    .ok_or(concat!("Input '", #name_str, "' is not an integer"))?;
            }
        }
        // bool
        "bool" => {
            quote! {
                let #name: bool = inputs.get(#index)
                    .ok_or(concat!("Could not get input '", #name_str, "'"))?
                    .as_bool()
                    .ok_or(concat!("Input '", #name_str, "' is not a boolean"))?;
            }
        }
        // &str
        "& str" => {
            quote! {
                let #name: &str = inputs.get(#index)
                    .ok_or(concat!("Could not get input '", #name_str, "'"))?
                    .as_str()
                    .ok_or(concat!("Input '", #name_str, "' is not a string"))?;
            }
        }
        _ => {
            panic!(
                "flow_function: unsupported parameter type '{type_str}' for input '{name_str}'. \
                 Supported types: &Value, Value, &Number, f64, i64, bool, &str.",
            );
        }
    }
}

fn check_return_type(return_type: &ReturnType) {
    assert_eq!(
        return_type.into_token_stream().to_string(),
        quote! { -> Result<(Option<Value>, RunAgain)>}.to_string(),
        "a 'flow_function' macro check failed:\n\
                                    implementation's return type does not match the \
                                    Implementation trait's run() method return type"
    );
}

fn generate_code(
    function_implementation: TokenStream,
    definition: &FunctionDefinition,
) -> TokenStream {
    let implementation: proc_macro2::TokenStream = function_implementation.clone().into();
    let implementation_ast = parse_macro_input!(function_implementation as syn::ItemFn);
    let implementation_name = &implementation_ast.sig.ident;

    check_return_type(&implementation_ast.sig.output);

    let input_mode = analyze_inputs(definition, &implementation_ast);

    let number_of_defined_inputs = definition.inputs.len();

    let input_number_check = quote! {
        if inputs.len() != #number_of_defined_inputs {
            flowcore::errors::bail!("'inputs' does not have the expected number of input values");
        }
    };

    let call_code = match &input_mode {
        InputMode::SliceMode(ident) => {
            quote! { #implementation_name(#ident) }
        }
        InputMode::TypedMode {
            extraction,
            call_args,
        } => {
            quote! {
                #extraction
                #implementation_name(#call_args)
            }
        }
    };

    let docs_comment = if definition.docs.is_empty() {
        quote! {}
    } else {
        let docs_file = &definition.docs;
        quote! {
            #[doc = include_str!(#docs_file)]
        }
    };

    let struct_name = format_ident!(
        "{}",
        FunctionDefinition::camel_case(&definition.name.clone())
    );

    let wasm_boilerplate = quote! {
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn alloc(size: usize) -> *mut std::os::raw::c_void {
            use std::mem;
            let mut buf = Vec::with_capacity(size);
            let ptr = buf.as_mut_ptr();
            mem::forget(buf);
            return ptr as *mut std::os::raw::c_void;
        }

        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn run_wasm(input_data_ptr: *mut std::os::raw::c_void, input_data_length: i32) -> i32 {
            use std::ptr::copy;
            let input_data: &[u8] = unsafe {
                std::slice::from_raw_parts(input_data_ptr as *mut u8,
                                      input_data_length as usize)
            };

            let inputs: Vec<serde_json::Value> = serde_json::from_slice(&input_data).unwrap();
            let object = #struct_name {};
            let result = flowcore::Implementation::run(&object, &inputs);

            let return_data = serde_json::to_vec(&result).unwrap();

            unsafe { copy(return_data.as_ptr(), input_data_ptr as *mut u8, return_data.len()); }

            return_data.len() as i32
        }
    };

    let gen = quote! {
        #[allow(unused_imports)]
        #wasm_boilerplate

        #[allow(clippy::unnecessary_wraps)]
        #implementation

        #docs_comment
        #[derive(Debug)]
        pub struct #struct_name;
        impl flowcore::Implementation for #struct_name {
            fn run(&self, inputs: &[serde_json::Value]) -> flowcore::errors::Result<(Option<serde_json::Value>, flowcore::RunAgain)> {
                #input_number_check
                #call_code
            }
        }

    };
    gen.into()
}
