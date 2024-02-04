use radix_engine::types::scrypto_decode;
use radix_runtime_fuzzer_common::{RadixRuntimeFuzzerTransaction};

use radix_engine::{system::bootstrap::Bootstrapper, transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig}, vm::{wasm::{DefaultWasmEngine, WasmValidatorConfigV1}, DefaultNativeVm, ScryptoVm, Vm}};
use scrypto_test::runner::TestRunnerBuilder;
use radix_engine_common::{data::scrypto, prelude::*};
use std::{fs::OpenOptions, io::{Read, Write}, time::Instant};
use radix_engine::vm::wasm_runtime::RadixRuntimeFuzzerInstruction;

fn main() {
    let input_dir = "/workspaces/develop/radix-runtime-fuzzer-test-cases/raw";
    let output_dir = "/workspaces/develop/radix-runtime-fuzzer-test-cases/extracted";

    // read all files in input_dir
    let entries = std::fs::read_dir(input_dir).unwrap();
    let mut txs_vec = Vec::new();
    for entry in entries {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let file_name = file_name.to_str().unwrap();
        let file_name = format!("{}/{}", input_dir, file_name);
        let file_data = std::fs::read(file_name).unwrap();
        let txs = RadixRuntimeFuzzerTransaction::vec_from_slice(&file_data).unwrap();
        // check if any txs has invokes non empty
        let mut invokes = 0;
        for tx in &txs {
            invokes += tx.invokes.len();
        }
        if invokes > 10 {
            txs_vec.push(txs);
        }
    }

    let txs_vec_data = scrypto_encode(&txs_vec).unwrap();
    std::fs::write(format!("{}/txs.bin", output_dir), txs_vec_data).unwrap();


    for (tx_id, txs) in txs_vec.iter().enumerate() {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(false)
            .open(format!("{}/{}.bin", output_dir, tx_id))
            .unwrap();
        file.write_all(&[tx_id as u8]).unwrap();
        for tx in txs {
            file.write_all(&scrypto_encode(&tx.instructions).unwrap()).unwrap();
            file.write_all(&scrypto_encode(&tx.invokes).unwrap()).unwrap();
        }
    }
}

