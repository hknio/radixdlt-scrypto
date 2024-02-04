use radix_engine::types::{manifest_decode, scrypto_decode, ManifestValue, ScryptoDecoder, ScryptoValue, ScryptoValueKind};
use radix_engine_common::constants::{SCRYPTO_SBOR_V1_MAX_DEPTH, SCRYPTO_SBOR_V1_PAYLOAD_PREFIX};
use radix_runtime_fuzzer_common::{RadixRuntimeFuzzerInput, RadixRuntimeFuzzerTransaction};
use transaction::model::InstructionV1;
use std::io::Read;
use radix_engine::vm::wasm_runtime::RadixRuntimeFuzzerInstruction;

fn main() {
    let file_name = std::env::args().nth(1).unwrap_or("txs.bin".to_string());
    let mut file = std::fs::File::open(file_name.clone()).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    let txs = RadixRuntimeFuzzerTransaction::vec_from_slice(&data).unwrap();
    for (tx_id, tx) in txs.iter().enumerate() {
        println!("TRANSACTION {}", tx_id);
        let instructions : Vec<InstructionV1> = manifest_decode(&tx.instructions).unwrap();
        println!("-- MANIFEST INSTRUCTION: {:?}", instructions);
        for instruction in instructions {
            println!("---- {:?}", instruction);
        }
        for (invoke_id, invoke) in tx.invokes.iter().enumerate() {
            println!("-- INVOKE {}", invoke_id);
            for instruction_data in invoke {
                let instruction : RadixRuntimeFuzzerInstruction = scrypto_decode(&instruction_data).unwrap();
                println!("---- {:?}", instruction);
            }
        }
    }
}

