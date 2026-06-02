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
use flowcore::model::name::HasName;

/// The `flow_function` macro definition
///
/// # Panics
///
/// Will panic if the file path of the source file where the macro was used cannot be determined.
#[proc_macro_attribute]
pub fn flow_function(_attr: TokenStream, implementation: TokenStream) -> TokenStream {
    let source_file = Span::call_site()
        .local_file()
        .expect("the 'flow' macro could not get the file path where macro was invoked");

    let mut definition_file_path = source_file.clone();
    definition_file_path.set_extension("toml");

    let function_definition = if definition_file_path.exists() {
        load_function_definition(&definition_file_path)
    } else {
        let impl_clone: proc_macro2::TokenStream = implementation.clone().into();
        let ast =
            syn::parse2::<ItemFn>(impl_clone).expect("flow_function: could not parse function");
        let def = generate_function_definition(&ast, &source_file);
        write_function_definition(&def, &definition_file_path);
        def
    };

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

/// Generate a `FunctionDefinition` from the function's signature and doc comments.
fn generate_function_definition(ast: &ItemFn, source_file: &Path) -> FunctionDefinition {
    let fn_name = ast.sig.ident.to_string();
    let name = fn_name
        .strip_prefix("inner_")
        .unwrap_or(&fn_name)
        .to_string();

    let source = source_file
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    // Extract description from doc comments
    let description = ast
        .attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                attr.meta.require_name_value().ok().and_then(|nv| {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    {
                        Some(s.value().trim().to_string())
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Check for docs .md file
    let mut docs_path = source_file.to_path_buf();
    docs_path.set_extension("md");
    let docs = if docs_path.exists() {
        docs_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Build inputs from typed parameters
    let inputs = build_inputs_from_signature(ast);

    // Default: single unnamed output (most common case)
    let outputs = flowcore::model::io::IOSet::default();

    let mut def = FunctionDefinition::default();
    def.name = name;
    def.source = source;
    def.docs = docs;
    def.description = description;
    def.build_type = "rust".to_string();
    def.inputs = inputs;
    def.outputs = outputs;
    def
}

/// Build an `IOSet` from the function's typed parameters.
fn build_inputs_from_signature(ast: &ItemFn) -> flowcore::model::io::IOSet {
    use flowcore::model::datatype::DataType;
    use flowcore::model::io::IO;

    let mut ios = Vec::new();

    for fn_arg in &ast.sig.inputs {
        if let FnArg::Typed(pat_type) = fn_arg {
            let param_name = match pat_type.pat.as_ref() {
                Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
                _ => String::new(),
            };

            let type_str = pat_type.ty.to_token_stream().to_string();
            let flow_type = rust_type_to_flow_type(&type_str);

            let mut io = IO::new(vec![DataType::from(flow_type.as_str())], "");
            io.set_name(param_name);
            ios.push(io);
        }
    }

    flowcore::model::io::IOSet::from(ios)
}

/// Map a Rust parameter type string to a flow type string.
fn rust_type_to_flow_type(type_str: &str) -> String {
    match type_str {
        "& Number" | "& serde_json :: Number" | "f64" | "i64" => "number".to_string(),
        "bool" => "boolean".to_string(),
        "& str" => "string".to_string(),
        _ => String::new(),
    }
}

/// Write a generated `FunctionDefinition` to a TOML file.
fn write_function_definition(def: &FunctionDefinition, path: &Path) {
    let toml_str = toml::to_string_pretty(def).unwrap_or_else(|e| {
        panic!("flow_function: could not serialize generated definition to TOML: {e}")
    });
    fs::write(path, toml_str).unwrap_or_else(|e| {
        panic!(
            "flow_function: could not write generated definition to '{}': {e}",
            path.display()
        )
    });
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

            // Validate parameter name matches TOML input name (skip if TOML name is empty/default)
            let toml_name = definition
                .inputs
                .get(i)
                .expect("input index out of bounds")
                .name();
            if !toml_name.is_empty() {
                let param_str = param_name.to_string();
                let toml_normalized = toml_name.replace('-', "_");
                assert_eq!(
                    param_str, toml_normalized,
                    "flow_function: parameter '{param_str}' at position {i} does not match \
                     TOML input name '{toml_name}' in function '{}'",
                    definition.name
                );
            }

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
