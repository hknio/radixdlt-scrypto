use radix_engine_interface::types::Level;
use radix_substate_store_queries::typed_substate_layout::PackageDefinition;
use std::path::PathBuf;
use scrypto_compiler::*;


pub fn build_with_coverage(path: PathBuf) -> (Vec<u8>, PackageDefinition) {
    let mut compiler_builder = ScryptoCompiler::builder();
    compiler_builder
        .manifest_path(path)
        .log_level(Level::Trace)
        .optimize_with_wasm_opt(None)
        .coverage();

    let flags = vec![
        "-Cllvm-args=-sanitizer-coverage-inline-8bit-counters",
        "-Cpasses=sancov-module",
        "-Cllvm-args=-sanitizer-coverage-level=3",
    ];

    // join flags with a special character
    let flags_env = 

    compiler_builder.env("CARGO_ENCODED_RUSTFLAGS", EnvironmentVariableAction::Set(flags.join("\x1f")));

    let build_results = compiler_builder
        .compile()
        .unwrap()
        .pop()
        .unwrap();
    (build_results.wasm.content, build_results.package_definition.content)    
}