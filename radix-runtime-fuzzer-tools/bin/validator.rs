use std::{io::Read, time::Instant};

use radix_runtime_fuzzer::FuzzRunner;
use radix_runtime_fuzzer_common::RadixRuntimeFuzzerTransaction;

// Validates file with transactions
fn main() {
    let file_name = std::env::args().nth(1).unwrap_or("txs.bin".to_string());
    let mut file = std::fs::File::open(file_name).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let txs = RadixRuntimeFuzzerTransaction::vec_from_slice(&data).unwrap(); 
    let mut runner = FuzzRunner::new();    
    let start = Instant::now();
    for tx in txs.clone() { 
        runner.execute(tx).expect_commit_success();
    }
    // that's optional
    let duration = start.elapsed().as_millis();
    if duration > 20 {
        panic!("Execution took too long: {}ms", duration);
    }
}

