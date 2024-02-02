use radix_engine::types::scrypto_decode;
use radix_runtime_fuzzer::{RadixRuntimeFuzzerTransaction};

use radix_engine::{system::bootstrap::Bootstrapper, transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig}, vm::{wasm::{DefaultWasmEngine, WasmValidatorConfigV1}, DefaultNativeVm, ScryptoVm, Vm}};
use scrypto_test::runner::TestRunnerBuilder;
use radix_engine_common::prelude::*;
use std::{io::{Read, Write}, time::Instant};
use radix_engine::vm::wasm_runtime::RadixRuntimeFuzzerInstruction;

fn main() {
    let file_name = std::env::args().nth(1).unwrap_or("tx_1.bin".to_string());

    // open and read file
    let mut file = std::fs::File::open(file_name).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let tx : RadixRuntimeFuzzerTransaction = scrypto_decode(&buffer).unwrap();
    for (index, invoke) in tx.invokes.iter().enumerate() {
        let mut file = std::fs::File::create(format!("invoke_{}.bin", index)).unwrap();
        for instruction_data in invoke {
            let _instruction : RadixRuntimeFuzzerInstruction = scrypto_decode(&instruction_data).unwrap();
            file.write_all(&instruction_data).unwrap();
        }
    }
    
}

