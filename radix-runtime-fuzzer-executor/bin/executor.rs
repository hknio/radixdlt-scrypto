use radix_engine::{types::scrypto_decode, vm::wasm_runtime::RadixRuntimeFuzzerInstruction};
use radix_runtime_fuzzer::RadixRuntimeFuzzerTransaction;

use radix_engine::{system::bootstrap::Bootstrapper, transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig}, vm::{wasm::{DefaultWasmEngine, WasmValidatorConfigV1}, DefaultNativeVm, ScryptoVm, Vm}};
use scrypto_test::runner::TestRunnerBuilder;
use radix_engine_common::prelude::*;
use std::{io::Write, time::Instant};

fn main() {
    let tx0 : RadixRuntimeFuzzerTransaction = scrypto_decode(include_bytes!("tx_0.bin")).unwrap();
    let tx1 : RadixRuntimeFuzzerTransaction = scrypto_decode(include_bytes!("tx_1.bin")).unwrap();
    let tx2 : RadixRuntimeFuzzerTransaction = scrypto_decode(include_bytes!("tx_2.bin")).unwrap();

    let mut test_runner = TestRunnerBuilder::new().without_trace().build();

    let mut txs = vec![tx0.clone(), tx1.clone(), tx2.clone()];
    for tx in &mut txs { 
        let recipt: scrypto_test::prelude::TransactionReceiptV1 = test_runner.execute_transaction(
            tx.get_executable(),
            CostingParameters::default(),
            ExecutionConfig::for_system_transaction(NetworkDefinition::simulator()),
        );
        recipt.expect_commit(true);
    }
    
    /*
    let snapshot = test_runner.create_snapshot();
    // save snapshot to file
    let mut file = std::fs::File::create("snapshot.bin").unwrap();
    file.write_all(scrypto_encode(&snapshot).unwrap().as_slice()).unwrap();
    */
    let start = Instant::now();
    for i in 0..100 {
        let mut txs = vec![tx0.clone(), tx1.clone(), tx2.clone()];
        for tx in &mut txs { 
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

    /*
    let mut instructions : Vec<RadixRuntimeFuzzerInstruction> = Vec::new();
    let invoke_data = include_bytes!("../invoke_0.bin");
    let mut decoder = ScryptoDecoder::new(invoke_data, SCRYPTO_SBOR_V1_MAX_DEPTH);
    while decoder.remaining_bytes() > 0 {
        decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX).unwrap();
        let instruction : RadixRuntimeFuzzerInstruction = decoder.decode().unwrap();
        instructions.push(instruction);
    }

    let mut tx1 = tx1.clone();
    tx1.invokes[0] = instructions.iter().map(|instruction| scrypto_encode(&instruction).unwrap()).collect();
    let recipt: scrypto_test::prelude::TransactionReceiptV1 = test_runner.execute_transaction(
        tx1.get_executable(),
        CostingParameters::default(),
        ExecutionConfig::for_system_transaction(NetworkDefinition::simulator()),
    );
    recipt.expect_commit(true);

    //let duration = start.elapsed();
    //println!("Time elapsed in ms: {:?}", duration.as_millis());
     */
}

