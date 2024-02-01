use radix_engine::types::scrypto_decode;
use radix_runtime_fuzzer::{RadixRuntimeFuzzerData, RADIX_RUNTIME_LOGGER};

use radix_engine::{system::bootstrap::Bootstrapper, transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig}, vm::{wasm::{DefaultWasmEngine, WasmValidatorConfigV1}, DefaultNativeVm, ScryptoVm, Vm}};
use scrypto_test::runner::TestRunnerBuilder;
use radix_engine_common::prelude::*;
use std::time::Instant;

fn main() {
    RADIX_RUNTIME_LOGGER.lock().unwrap().disable();

    let tx0 : RadixRuntimeFuzzerData = scrypto_decode(include_bytes!("tx_0.bin")).unwrap();
    let tx1 : RadixRuntimeFuzzerData = scrypto_decode(include_bytes!("tx_1.bin")).unwrap();
    let tx2 : RadixRuntimeFuzzerData = scrypto_decode(include_bytes!("tx_2.bin")).unwrap();

    let mut test_runner = TestRunnerBuilder::new().without_trace().build();

    let txs = vec![tx0.clone(), tx1.clone(), tx2.clone()];
    for tx in txs { 
        let recipt: scrypto_test::prelude::TransactionReceiptV1 = test_runner.execute_transaction(
            tx.get_executable(),
            CostingParameters::default(),
            ExecutionConfig::for_system_transaction(NetworkDefinition::simulator()),
        );
        recipt.expect_commit(true);
    }

    let start = Instant::now();
    for i in 0..10000 {
        let txs = vec![tx2.clone()];
        for tx in txs { 
            let recipt: scrypto_test::prelude::TransactionReceiptV1 = test_runner.execute_transaction(
                tx.get_executable(),
                CostingParameters::default(),
                ExecutionConfig::for_system_transaction(NetworkDefinition::simulator()),
            );
            recipt.expect_commit(true);
        }
    }
    let duration = start.elapsed();
    println!("Time elapsed in ms: {:?}", duration.as_millis());

    //run_fuzz(vec![tx0, tx1, tx2]);
    //run_fuzz(include_bytes!("tx_2.bin").to_vec());
    //run_fuzz(include_bytes!("tx_3.bin").to_vec());
/*    run_fuzz(include_bytes!("tx_1.bin").to_vec());
    run_fuzz(include_bytes!("tx_2.bin").to_vec());
    run_fuzz(include_bytes!("tx_3.bin").to_vec());
    run_fuzz(include_bytes!("tx_4.bin").to_vec());  */
}

