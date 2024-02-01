extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ImplItem, Pat, WhereClause, WherePredicate};

#[proc_macro_attribute]
pub fn runtime_fuzzer(_attrs: proc_macro::TokenStream, item: proc_macro::TokenStream) -> TokenStream {
    // Parse the input into a syntax tree
    let mut input = parse_macro_input!(item as syn::ItemImpl);
    
    // Iterate over the items and modify only the methods
    let mut exec_func = String::new();

    exec_func += "fn execute_func(&mut self, func_id: u8, fuzz_args: Vec<Vec<u8>>) -> Result<(), ()> {\n";
    exec_func += "match func_id {\n";

    let mut func_id = 1;
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
                    // Skip `self` and other non-typed arguments
                    None
                }
            }).collect();


            // Create a new logging statement
            //let log_statement = quote! {
            //    println!("Function {} called with args: {:?}", stringify!(#method_name), (#(#args),*));
            //};

            let mut arg_id : u8 = 0;
            exec_func += &format!("{} => {{\n", func_id);
            exec_func += &format!("if fuzz_args.len() != {} {{\n", args_and_types.len());
            exec_func += &format!("panic!(\"Wrong number of arguments for function {} (expected {}, got {{}})\", fuzz_args.len());\n", method_name, args_and_types.len());
            exec_func += "}\n";
            for (arg, arg_type) in args_and_types.iter() {
                exec_func += &format!("let {}: {} = scrypto_decode(&fuzz_args[{}]).unwrap();\n", arg, arg_type, arg_id);                
                arg_id += 1;
            }
            exec_func += &format!("self.{}({}).map(|_value| ()).map_err(|_error| ())?;", method_name, args_and_types.iter().map(|(arg, _)| arg.to_string()).collect::<Vec<String>>().join(", "));
            exec_func += "},\n";

            let mut pre_exec = String::new();
            pre_exec += "{\n";
            pre_exec += "let mut fuzz_log_data : Vec<Vec<u8>> = Vec::new();\n";
            for (arg, arg_type) in args_and_types.iter() {
                pre_exec += &format!("fuzz_log_data.push(scrypto_encode(&{}).unwrap());\n", arg);
            }
            pre_exec += &format!("RADIX_RUNTIME_LOGGER.lock().unwrap().runtime_call_start(String::from(\"{}\"), {}, fuzz_log_data);\n", method_name, func_id);
            pre_exec += "}\n";            
            let pre_exec: proc_macro2::TokenStream = syn::parse_str(&pre_exec).unwrap();

            let mut post_exec = String::new();
            post_exec += "{\n";
            post_exec += "let mut fuzz_log_data : Vec<Vec<u8>> = Vec::new();\n";
            post_exec += "if result.is_ok() {\n";
            post_exec += "fuzz_log_data.push(scrypto_encode(&result.as_ref().unwrap()).unwrap());\n";
            post_exec += "} else {\n";
            post_exec += "fuzz_log_data.push(Vec::new());\n";
            post_exec += "}\n";
            post_exec += &format!("RADIX_RUNTIME_LOGGER.lock().unwrap().runtime_call_end(String::from(\"{}\"), {}, fuzz_log_data);\n", method_name, func_id);
            post_exec += "}\n";
            let post_exec: proc_macro2::TokenStream = syn::parse_str(&post_exec).unwrap();


            let original_body = &method.block;
            let combined_block = quote! {
                {
                    #pre_exec
                    let result = (|| #original_body)();
                    #post_exec
                    result
                }
            };
            func_id += 1;

            // Parse the combined block and assign it to the method
            method.block = syn::parse2(combined_block).expect("Failed to parse new method body");
        }
    });

    exec_func += "_ => { return Err(()); }\n";
    exec_func += "};\n";
    exec_func += "Ok(())\n";
    exec_func += "}\n";
    
    let exec_func: syn::ImplItem = syn::parse_str(&exec_func).unwrap();
    

    let mut new_impl = input.clone(); // Clone the original implementation

    // Extract the type name and trait name from the cloned block
    let type_name = &new_impl.self_ty;
    let trait_name = if let Some((_, path, _)) = &mut new_impl.trait_ {
        let original_trait_name = path.segments.last().unwrap().ident.clone();
        path.segments.last_mut().unwrap().ident = syn::Ident::new("RadixRuntimeFuzzer", path.segments.last().unwrap().ident.span());
        original_trait_name
    } else {
        return syn::Error::new_spanned(&new_impl, "Expected an impl block for a trait").to_compile_error().into();
    };

    // Prepare the new where predicate for the cloned block
    let new_where_predicate: WherePredicate = parse_quote! { #type_name: #trait_name };

    // Ensure that there is a where clause to add the predicate to in the cloned block
    if new_impl.generics.where_clause.is_none() {
        new_impl.generics.where_clause = Some(WhereClause {
            where_token: Default::default(),
            predicates: syn::punctuated::Punctuated::new(),
        });
    }

    // Add the new where predicate to the cloned block
    new_impl.generics.where_clause.as_mut().unwrap().predicates.push(new_where_predicate);
    new_impl.items = vec![exec_func];   


    // Reconstruct the `impl` block with the modified methods
    let output = quote! {
        use std::fs::OpenOptions;
        use std::io::prelude::*;

        #input

        #new_impl
    };

    output.into()
}
