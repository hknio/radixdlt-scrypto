use radix_runtime_fuzzer_common::RadixRuntimeFuzzerTransaction;
use std::io::Read;
use std::time::Instant;
use radix_runtime_fuzzer::FuzzRunner;

fn main() {
    // read file from argument or use default txs.bin
    let file_name = std::env::args().nth(1).unwrap_or("txs.bin".to_string());
    let mut file = std::fs::File::open(file_name.clone()).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let mut txs = RadixRuntimeFuzzerTransaction::vec_from_slice(&data).unwrap(); 
    let mut runner = FuzzRunner::new();
    
    println!("Executing {} transactions from {} 10000x times", txs.len(), file_name);
    let start = Instant::now();
    for _ in 0..10000 {
        runner.reset();
        for tx in txs.clone() { 
            runner.execute(tx).expect_commit_success();
        }
    }
    let duration = start.elapsed();
    println!("Time elapsed in ms: {:?}", duration.as_millis());
}

