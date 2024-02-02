use radix_engine::{
    types::scrypto_decode,
    vm::{wasm_runtime::RadixRuntimeFuzzerInstruction, NoExtension},
};
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use radix_runtime_fuzzer::RadixRuntimeFuzzerTransaction;

use radix_engine::{
    system::bootstrap::Bootstrapper,
    transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig},
    vm::{
        wasm::{DefaultWasmEngine, WasmValidatorConfigV1},
        DefaultNativeVm, ScryptoVm, Vm,
    },
};
use radix_engine_common::prelude::*;
use scrypto_test::runner::{TestRunner, TestRunnerBuilder, TestRunnerSnapshot};

struct FuzzRunner {
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

    pub fn reset(&mut self) {
        self.test_runner = TestRunnerBuilder::new()
            .without_trace()
            .build_from_snapshot(self.snapshot.clone());
    }

    pub fn execute(&mut self, mut tx: RadixRuntimeFuzzerTransaction) -> bool {
        self.test_runner
            .execute_transaction(
                tx.get_executable(),
                CostingParameters::default(),
                ExecutionConfig::for_notarized_transaction(NetworkDefinition::simulator()),
            )
            .is_commit_success()
    }
}
