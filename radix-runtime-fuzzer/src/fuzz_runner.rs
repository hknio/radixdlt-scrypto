use radix_engine::{
    transaction::TransactionReceiptV1, types::scrypto_decode, vm::NoExtension
};
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use radix_runtime_fuzzer_common::RadixRuntimeFuzzerTransaction;

use radix_engine::transaction::{CostingParameters, ExecutionConfig};
use radix_engine_common::prelude::*;
use scrypto_test::runner::{TestRunner, TestRunnerBuilder, TestRunnerSnapshot};
use radix_engine::vm::wasm::FuzzerEngine;
pub struct FuzzRunner {
    snapshot: TestRunnerSnapshot,
    test_runner: TestRunner<NoExtension, InMemorySubstateDatabase>,
}

impl FuzzRunner {
    pub fn new() -> Self {
        // snapshot is used to avoid generating coverage data from bootstrap
        let snapshot: TestRunnerSnapshot = scrypto_decode(include_bytes!("snapshot.bin")).unwrap();
        let test_runner = TestRunnerBuilder::new()
            .without_trace()
            .build_from_snapshot(snapshot.clone());
        Self {
            snapshot,
            test_runner,
        }
    }

    pub fn create_snapshot(&self) -> TestRunnerSnapshot {
        self.test_runner.create_snapshot()
    }

    pub fn update_snapshot(&mut self) {
        self.snapshot = self.test_runner.create_snapshot();
    }

    pub fn save_snapshot(&mut self) {
        let snapshot = self.create_snapshot();
        let snapshot_data = scrypto_encode(&snapshot).unwrap();
        let snapshot_hash = hash(&snapshot_data);
        std::fs::write(&format!("snapshot_{}.bin", snapshot_hash.to_string()), snapshot_data).unwrap();
        println!("Snapshot saved: snapshot_{}.bin", snapshot_hash.to_string());
    }

    pub fn reset(&mut self) {
        self.test_runner.restore_snapshot(self.snapshot.clone());
    }

    pub fn execute(&mut self, mut tx: RadixRuntimeFuzzerTransaction) -> TransactionReceiptV1 {
        FuzzerEngine::set_invokes(tx.invokes.clone());
        self.test_runner
            .execute_transaction(
                tx.get_executable(),
                CostingParameters::default(),
                ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator()),
            )
    }
}
