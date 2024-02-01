use std::{fs::OpenOptions, io::Write};

use once_cell::sync::Lazy;
use radix_engine::{system::bootstrap::Bootstrapper, transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig}, vm::{wasm::{DefaultWasmEngine, WasmValidatorConfigV1}, DefaultNativeVm, ScryptoVm, Vm}};
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use scrypto_test::runner::TestRunnerBuilder;
use transaction::{model::{ExecutionContext, InstructionV1}, prelude::Executable};
use radix_engine_common::{data::scrypto, prelude::*};
use radix_runtime_fuzzer::*;

pub fn run_fuzz(txs: Vec<RadixRuntimeFuzzerData>) {
    let mut test_runner = TestRunnerBuilder::new().without_trace().build();
    for tx in txs { 
        let recipt: scrypto_test::prelude::TransactionReceiptV1 = test_runner.execute_transaction(
            tx.get_executable(),
            CostingParameters::default(),
            ExecutionConfig::for_system_transaction(NetworkDefinition::simulator()),
        );
        recipt.expect_commit(true);
    }
}
