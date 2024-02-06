#![no_main]

#[cfg(feature="libfuzzer")]
use libfuzzer_sys::fuzz_target;
#[cfg(feature="honggfuzz")]
use honggfuzz::fuzz;

use once_cell::sync::Lazy;
use radix_engine::{types::scrypto_decode, vm::{wasm_runtime::RadixRuntimeFuzzerInstruction, NoExtension}};
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use radix_runtime_fuzzer_common::{RadixRuntimeFuzzerInput, RadixRuntimeFuzzerTransaction};

use radix_engine::{system::bootstrap::Bootstrapper, transaction::{execute_and_commit_transaction, CostingParameters, ExecutionConfig}, vm::{wasm::{DefaultWasmEngine, WasmValidatorConfigV1}, DefaultNativeVm, ScryptoVm, Vm}};
use scrypto_test::runner::{TestRunner, TestRunnerBuilder, TestRunnerSnapshot};
use radix_engine_common::{data::scrypto, prelude::*};

use radix_runtime_fuzzer::FuzzRunner;
use transaction::model::InstructionV1;
use transaction::model::extract_references;

struct Fuzzer {
    runner: FuzzRunner,
    txs: Vec<Vec<RadixRuntimeFuzzerTransaction>>,
    executed_txs: usize,
}

impl Fuzzer {
    fn new() -> Self {
        let txs_data = include_bytes!("../../radix-runtime-fuzzer-test-cases/extracted/txs.bin");
        let txs: Vec<Vec<RadixRuntimeFuzzerTransaction>> = scrypto_decode(txs_data).unwrap();

        let snapshot_data = include_bytes!("../../snapshot.bin");
        let snapshot = scrypto_decode(snapshot_data).unwrap();

        let mut runner = FuzzRunner::from_snapshot(snapshot);

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
        let mut txs_instructions = Vec::new();
        let mut decoder = ManifestDecoder::new(&data[1..], MANIFEST_SBOR_V1_MAX_DEPTH - 4);
        while decoder.remaining_bytes() > 0 && tx_id < txs.len() {
            let mut instructions = Vec::new();
            while decoder.peek_byte().map_err(|_| ())? != 0xFF {
                let instruction = decoder.decode::<InstructionV1>().map_err(|_| ())?;
                instructions.push(instruction);
            }
            decoder.read_byte().map_err(|_| ())?;
            let mut invokes : Vec<RadixRuntimeFuzzerInput> = Vec::new();
            invokes.push(Vec::new());
            while decoder.peek_byte().map_err(|_| ())? != 0xFF {
                let instruction = decoder.decode::<RadixRuntimeFuzzerInstruction>().map_err(|_| ())?;
                invokes.last_mut().unwrap().push(scrypto_encode(&instruction).unwrap());
                if let RadixRuntimeFuzzerInstruction::Return(_) = instruction {
                    invokes.push(Vec::new());
                }    
            }
            decoder.read_byte().map_err(|_| ())?;
            invokes.pop();

            txs_instructions.push(instructions);
            txs[tx_id].invokes = invokes;
            tx_id += 1;
        }

        for (tx_id, instructions) in txs_instructions.iter_mut().enumerate() {
            txs[tx_id].instructions = manifest_encode(instructions).unwrap();
            txs[tx_id].references = extract_references(&txs[tx_id].instructions[1..], traversal::ExpectedStart::Value);
        }

        let mut is_ok = txs.len() > 0;
        for tx in txs {
            if !self.runner.execute(tx).is_commit_success() {
                is_ok = false;
            } else {
                self.executed_txs += 1;
            }
        }
        self.runner.reset();

        if self.executed_txs % 1000 == 0 {
            println!("Executed {} transactions", self.executed_txs);
        }

        Ok(is_ok)
    }
}


#[cfg(feature="libfuzzer")]
fuzz_target!(|data: &[u8]| {
    unsafe {
        pub static mut FUZZER: Lazy<Fuzzer> = Lazy::new(|| Fuzzer::new());
        FUZZER.run(data);
    }
});

#[cfg(feature="honggfuzz")]
fn main() {
    let mut fuzzer = Fuzzer::new();
    loop {
        fuzz!(|data: &[u8]| {
            fuzzer.run(data);
        });
    }
}

