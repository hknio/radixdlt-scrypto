#![no_main]

use libfuzzer_sys::fuzz_target;
use once_cell::sync::Lazy;
use radix_engine::{types::scrypto_decode, vm::{wasm_runtime::RadixRuntimeFuzzerInstruction, NoExtension}};
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use radix_runtime_fuzzer::{RadixRuntimeFuzzerTransaction};

use radix_engine::{system::bootstrap::Bootstrapper, transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig}, vm::{wasm::{DefaultWasmEngine, WasmValidatorConfigV1}, DefaultNativeVm, ScryptoVm, Vm}};
use scrypto_test::runner::{TestRunner, TestRunnerBuilder, TestRunnerSnapshot};
use radix_engine_common::prelude::*;

struct Fuzzer {
    test_runner: TestRunner<NoExtension, InMemorySubstateDatabase>,
    tx: RadixRuntimeFuzzerTransaction
}

impl Fuzzer {
    fn new() -> Self {
        let snapshot : TestRunnerSnapshot = scrypto_decode(include_bytes!("../snapshot.bin")).unwrap();        
        let mut test_runner = TestRunnerBuilder::new().without_trace().build_from_snapshot(snapshot);

        test_runner.disable_commit();

        Self {
            test_runner,
            tx: scrypto_decode(include_bytes!("tx_2.bin")).unwrap()
        }
    }

    fn run(&mut self, data: &[u8]) -> Result<bool, ()> {
        let mut instructions : Vec<RadixRuntimeFuzzerInstruction> = Vec::new();
        let mut decoder = ScryptoDecoder::new(data, SCRYPTO_SBOR_V1_MAX_DEPTH);
        while decoder.remaining_bytes() > 0 {
            decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX).is_ok();
            let instruction = decoder.decode::<RadixRuntimeFuzzerInstruction>();
            if let Ok(instruction) = instruction {
                instructions.push(instruction);
            }
        }

        let mut tx = self.tx.clone();
        tx.invokes[0] = instructions.iter().map(|instruction| scrypto_encode(&instruction).unwrap()).collect();
        let recipt: scrypto_test::prelude::TransactionReceiptV1 = self.test_runner.execute_transaction(
            tx.get_executable(),
            CostingParameters::default(),
            ExecutionConfig::for_system_transaction(NetworkDefinition::simulator()),
        );
        Ok(recipt.is_commit_success())             
    }
}


fuzz_target!(|data: &[u8]| {
    unsafe {
        pub static mut FUZZER: Lazy<Fuzzer> = Lazy::new(|| Fuzzer::new());
        FUZZER.run(data).is_ok();    
    }
});

