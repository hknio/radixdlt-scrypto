use std::{io::Read, time::Instant};

use radix_engine::types::scrypto_encode;
use radix_runtime_fuzzer::FuzzRunner;
use radix_runtime_fuzzer_common::RadixRuntimeFuzzerTransaction;
use radix_engine::prelude::scrypto_decode;

// Validates file with transactions
fn main() {
    let file_name = std::env::args().nth(1).unwrap_or("txs.bin".to_string());
    let mut file = std::fs::File::open(file_name).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let txs = RadixRuntimeFuzzerTransaction::vec_from_slice(&data).unwrap(); 

    let file_name = "/workspaces/develop/snapshot.bin".to_string();
    let mut file = std::fs::File::open(file_name).unwrap();
    let mut snapshot_data = Vec::new();
    file.read_to_end(&mut snapshot_data).unwrap();

    let snapshot = scrypto_decode(&snapshot_data).unwrap();
    let mut runner = FuzzRunner::from_snapshot(snapshot);   

    for tx in txs.clone() { 
        runner.execute(tx).expect_commit_success();
    }

    let mut snapshot = runner.create_snapshot();
    snapshot.next_private_key += 100;
    snapshot.next_transaction_nonce += 100;

    let snapshot_data = scrypto_encode(&snapshot).unwrap();
    std::fs::write("/workspaces/develop/snapshot.bin", snapshot_data).unwrap();
}

