#![no_main]

use libfuzzer_sys::fuzz_target;
use once_cell::sync::Lazy;
use radix_engine::{types::scrypto_decode, vm::{wasm_runtime::RadixRuntimeFuzzerInstruction, NoExtension}};
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use radix_runtime_fuzzer_common::{RadixRuntimeFuzzerInput, RadixRuntimeFuzzerTransaction};

use radix_engine::{system::bootstrap::Bootstrapper, transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig}, vm::{wasm::{DefaultWasmEngine, WasmValidatorConfigV1}, DefaultNativeVm, ScryptoVm, Vm}};
use scrypto_test::runner::{TestRunner, TestRunnerBuilder, TestRunnerSnapshot};
use radix_engine_common::{data::scrypto, prelude::*};

use radix_runtime_fuzzer::FuzzRunner;
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

    fn run(&mut self, data: &[u8]) -> Result<bool, ()> {
        if data.len() < 4 {
            return Err(());
        }

        let tx_id = data[0] % self.txs.len() as u8;

        let mut txs = self.txs[tx_id as usize].clone();
        let mut tx_id = 0;

        let mut decoder = ScryptoDecoder::new(&data[1..], SCRYPTO_SBOR_V1_MAX_DEPTH);
        let mut invokes : Vec<RadixRuntimeFuzzerInput> = Vec::new();
        invokes.push(Vec::new());
        while decoder.remaining_bytes() > 0 && tx_id < txs.len() {
            let _ = decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX).is_ok();
            let instruction = decoder.decode::<RadixRuntimeFuzzerInstruction>().map_err(|_| ())?;
            invokes.last_mut().unwrap().push(scrypto_encode(&instruction).unwrap());
            if let RadixRuntimeFuzzerInstruction::Return(_) = instruction {
                if txs[tx_id].invokes.len() == invokes.len() {
                    txs[tx_id].invokes = invokes.clone();
                    invokes.clear();
                    tx_id += 1;
                }
                invokes.push(Vec::new());
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
        if let Ok(status) = FUZZER.run(data) {
            //println!("Status: {}", status);
        }
    }
});

