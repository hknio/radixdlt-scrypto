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

    compiler_builder.env("CARGO_ENCODED_RUSTFLAGS", EnvironmentVariableAction::Set("-Cllvm-args=-sanitizer-coverage-inline-8bit-counters\x1f-Cpasses=sancov-module\x1f-Cllvm-args=-sanitizer-coverage-level=3".to_string()));

    let build_results = compiler_builder
        .compile()
        .unwrap()
        .pop()
        .unwrap();
    (build_results.wasm.content, build_results.package_definition.content)    
}