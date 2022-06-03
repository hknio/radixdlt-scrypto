// This is optional, as you may choose to use std for testing only.
#![no_std]

use radix_engine::engine::TransactionExecutor;
use radix_engine::ledger::*;
use radix_engine::model::extract_package;
use radix_engine::wasm::DefaultWasmEngine;
use scrypto::call_data;
use scrypto::prelude::*;
use transaction::builder::TransactionBuilder;

#[test]
fn test_say_hello() {
    // Set up environment.
    let mut substate_store = InMemorySubstateStore::with_bootstrap();
    let mut wasm_engine = DefaultWasmEngine::new();
    let mut executor = TransactionExecutor::new(&mut substate_store, &mut wasm_engine, true);
    let package = extract_package(include_package!("no_std").to_vec()).unwrap();
    let package_address = executor.publish_package(package).unwrap();

    // Test the `say_hello` function.
    let transaction1 = TransactionBuilder::new()
        .call_function(package_address, "NoStd", call_data!(say_hello()))
        .build_manifest();
    let signers = vec![];

    let receipt1 = executor.execute(&transaction1).unwrap();
    assert!(receipt1.result.is_ok());
}
