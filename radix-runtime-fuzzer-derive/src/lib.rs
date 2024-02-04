extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ImplItem, Pat, WhereClause, WherePredicate};
use heck::ToUpperCamelCase;

#[proc_macro_attribute]
pub fn radix_runtime_fuzzer(_attrs: proc_macro::TokenStream, item: proc_macro::TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as syn::ItemImpl);

    // Generate RadixRuntimeFuzzerInstruction enum
    let mut enum_def = String::new();
    enum_def += "#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]\n";
    enum_def += "pub enum RadixRuntimeFuzzerInstruction {\n";
    enum_def += "#[sbor(discriminator(0))]";
    enum_def += "Return(Vec<u8>),\n";
    
    // Generate execute_instructions function which executes RadixRuntimeFuzzerInstruction
    let mut exec_func = String::new();
    exec_func += "fn execute_instructions(&mut self, instructions : &Vec<Vec<u8>>) -> Result<Vec<u8>, ()> {\n";
    exec_func += "for instruction_data in instructions {\n";
    exec_func += "let instruction : RadixRuntimeFuzzerInstruction = scrypto_decode(&instruction_data).unwrap();\n";
    exec_func += "match instruction {\n";
    exec_func += "RadixRuntimeFuzzerInstruction::Return(data) => {\n";
    exec_func += "return Ok(data)\n";
    exec_func += "},\n";

    let mut func_id = 1; // 0 is reserved for return
    input.items.iter_mut().for_each(|item| {
        if let ImplItem::Method(method) = item {
            let method_name = &method.sig.ident;

            if method_name.to_string() == "consume_wasm_execution_units" {
                return;
            }

            let args_and_types: Vec<(TokenStream2, TokenStream2)> = method.sig.inputs.iter().filter_map(|arg| {
                if let syn::FnArg::Typed(pat_type) = arg {
                    if let Pat::Ident(ident_pattern) = pat_type.pat.as_ref() {
                        let ident = &ident_pattern.ident;
                        let arg_type = &pat_type.ty;
                        Some((quote! { #ident }, quote! { #arg_type }))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }).collect();

            let enum_name = method_name.to_string().to_upper_camel_case();
            enum_def += &format!("#[sbor(discriminator({}))]\n", func_id);
            enum_def += &format!("{}({}),\n", enum_name, args_and_types.iter().map(|(_, arg_type)| arg_type.to_string()).collect::<Vec<String>>().join(", "));

            exec_func += &format!("RadixRuntimeFuzzerInstruction::{}({}) => {{\n", enum_name, args_and_types.iter().map(|(arg, _)| arg.to_string()).collect::<Vec<String>>().join(", "));
            exec_func += &format!("self.{}({}).map_err(|_error| ())?;\n", method_name, args_and_types.iter().map(|(arg, _)| arg.to_string()).collect::<Vec<String>>().join(", "));
            exec_func += "},\n";

            func_id += 1;

            if cfg!(not(feature="radix_runtime_logger")) {
                return;
            }            

            // log method arguments
            let mut pre_exec = String::new();
            pre_exec += "{\n";
            pre_exec += &format!("let mut fuzz_log_data_enum = RadixRuntimeFuzzerInstruction::{}({});\n", enum_name, args_and_types.iter().map(|(arg, _)| arg.to_string() + ".clone()").collect::<Vec<String>>().join(", "));
            pre_exec += "let mut fuzz_log_data = scrypto_encode(&fuzz_log_data_enum).unwrap();\n";
            pre_exec += "radix_runtime_logger!(runtime_call_start(fuzz_log_data));\n";
            pre_exec += "}\n";            
            let pre_exec: proc_macro2::TokenStream = syn::parse_str(&pre_exec).expect("Failed to parse pre_exec");

            // log method return value
            let mut post_exec = String::new();
            post_exec += "{\n";
            post_exec += "let mut fuzz_log_data : Option<Vec<u8>> = None;\n";
            post_exec += "if result.is_ok() {\n";
            post_exec += "fuzz_log_data = Some(scrypto_encode(&result.as_ref().unwrap()).unwrap());\n";
            post_exec += "}\n";
            post_exec += "radix_runtime_logger!(runtime_call_end(fuzz_log_data));\n";
            post_exec += "}\n";
            let post_exec: proc_macro2::TokenStream = syn::parse_str(&post_exec).expect("Failed to parse post_exec");


            let original_body = &method.block;
            let combined_block = quote! {
                {
                    #pre_exec
                    let result = (|| #original_body)();
                    #post_exec
                    result
                }
            };
            method.block = syn::parse2(combined_block).expect("Failed to parse new method body");
        }
    });

    exec_func += "};\n";
    exec_func += "};\n";
    exec_func += "Err(())\n";
    exec_func += "}\n";

    if cfg!(feature="radix_runtime_fuzzing") {
        let exec_func: syn::ImplItem = syn::parse_str(&exec_func).expect("Failed to parse exec_func");
        input.items.push(exec_func);
    }
    
    enum_def += "}\n";
    let enum_def: syn::ItemEnum = syn::parse_str(&enum_def).expect("Failed to parse enum_def");

    // Reconstruct the `impl` block with the modified methods
    let output = quote! {
        #enum_def
        #input
    };

    output.into()
}


