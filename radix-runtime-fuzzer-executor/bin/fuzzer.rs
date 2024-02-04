#![no_main]

use libfuzzer_sys::fuzz_target;
use once_cell::sync::Lazy;
use radix_engine::{types::scrypto_decode, vm::{wasm_runtime::RadixRuntimeFuzzerInstruction, NoExtension}};
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use radix_runtime_fuzzer::{RadixRuntimeFuzzerInput, RadixRuntimeFuzzerTransaction};

use radix_engine::{system::bootstrap::Bootstrapper, transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig}, vm::{wasm::{DefaultWasmEngine, WasmValidatorConfigV1}, DefaultNativeVm, ScryptoVm, Vm}};
use scrypto_test::runner::{TestRunner, TestRunnerBuilder, TestRunnerSnapshot};
use radix_engine_common::prelude::*;

use radix_runtime_fuzzer_executor::FuzzRunner;
use transaction::model::InstructionV1;

struct Fuzzer {
    runner: FuzzRunner,
    txs: Vec<Vec<RadixRuntimeFuzzerTransaction>>,
    executed_txs: usize,
}

impl Fuzzer {
    fn new() -> Self {
        let txs_data = include_bytes!("/workspaces/develop/radix-runtime-fuzzer-test-cases/extracted/txs.bin");
        let txs: Vec<Vec<RadixRuntimeFuzzerTransaction>> = scrypto_decode(txs_data).unwrap();
        let mut runner = FuzzRunner::new();

        /*
        for tx in &txs {
            for tx in tx {
                runner.execute(tx.clone());
            }
        }
         */
        //runner.update_snapshot();

        Self {
            runner,
            txs,
            executed_txs: 0
        }
    }

    fn validate_invokes(&self, tx: &RadixRuntimeFuzzerTransaction) -> bool {
        for invoke in &tx.invokes {
            for instruction_data in invoke {
                if scrypto_decode::<RadixRuntimeFuzzerInstruction>(&instruction_data).is_err() {
                    return false;
                }
            }
        }
        true
    }

    fn validate_instructions(&self, tx: &RadixRuntimeFuzzerTransaction) -> bool {
        manifest_decode::<Vec<InstructionV1>>(&tx.instructions).is_ok()
    }

    fn run(&mut self, data: &[u8]) -> Result<bool, ()> {
        if data.len() < 4 {
            return Err(());
        }

        let tx_id = data[0];
        if tx_id as usize >= self.txs.len() {
            return Err(());
        }

        let mut txs = self.txs[tx_id as usize].clone();
        let mut tx_id = 0;

        let mut decoder = ScryptoDecoder::new(&data[1..], SCRYPTO_SBOR_V1_MAX_DEPTH);
        while decoder.remaining_bytes() > 0 && tx_id < txs.len() {
            decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX).map_err(|_| ())?;
            let instructions = decoder.decode::<Vec<u8>>().map_err(|_| ())?;
            decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX).map_err(|_| ())?;
            let invokes = decoder.decode::<Vec<RadixRuntimeFuzzerInput>>().map_err(|_| ())?;
            txs[tx_id].instructions = instructions;
            txs[tx_id].invokes = invokes;
            tx_id += 1;
        }

        for tx in &txs {
            if !self.validate_invokes(tx) || !self.validate_instructions(tx) {
                return Err(());
            }
        }

        let mut is_ok = txs.len() > 0;
        for tx in txs {
            if !self.runner.execute(tx).is_commit_success() {
                is_ok = false;
            } else {
                self.executed_txs += 1;
            }
        }
        /*
        if self.executed_txs > 100 {
            self.runner.reset();
            self.executed_txs = 0;
        }
         */
        Ok(is_ok)
    }
}


fuzz_target!(|data: &[u8]| {
    unsafe {
        pub static mut FUZZER: Lazy<Fuzzer> = Lazy::new(|| Fuzzer::new());
        pub static CRASH_DATA: &[u8; 8784] = include_bytes!("../crash-0bdb2efece8bba2625c4d59fe85f076cb947d51d");
        if FUZZER.run(data).is_ok() {
            FUZZER.run(CRASH_DATA);
            //println!("Success");
        }
    }
});

