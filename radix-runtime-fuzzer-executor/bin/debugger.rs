use radix_engine::types::{manifest_decode, scrypto_decode, ManifestValue, ScryptoDecoder, ScryptoValue, ScryptoValueKind};
use radix_engine_common::constants::{SCRYPTO_SBOR_V1_MAX_DEPTH, SCRYPTO_SBOR_V1_PAYLOAD_PREFIX};
use radix_runtime_fuzzer::{RadixRuntimeFuzzerInput, RadixRuntimeFuzzerTransaction, RadixRuntimeFuzzingInvokes, RADIX_RUNTIME_FUZZING_INVOKES};
use transaction::model::{InstructionV1, InstructionsV1};
use std::{io::Read, mem};
use std::time::Instant;
use radix_engine::vm::{wasm_runtime::RadixRuntimeFuzzerInstruction};

use radix_runtime_fuzzer_executor::FuzzRunner;
use sbor::{Decoder, Value};

fn process_invoke(input: &RadixRuntimeFuzzerInput, depth: usize) {
    for raw_instruction in input {
        let invoke_instruction: RadixRuntimeFuzzerInstruction = scrypto_decode(&raw_instruction).unwrap();
        println!("{} {:?}", "-".repeat(depth), invoke_instruction);
        let instruction: ScryptoValue = scrypto_decode(&raw_instruction).unwrap();
        if let ScryptoValue::Enum { fields, .. } = &instruction {
            let last_field = fields.last();
            if last_field.is_none() {
                continue;
            }
            if let ScryptoValue::Array { element_value_kind: ScryptoValueKind::U8, elements } = last_field.unwrap() {
                let elements = elements.iter().map(|e| {
                    if let ScryptoValue::U8 { value } = e {
                        *value
                    } else {
                        panic!("Unexpected instruction type");
                    }
                }).collect::<Vec<u8>>();
                if let Ok(invoke) = RADIX_RUNTIME_FUZZING_INVOKES.get_invoke_from_slice(&elements) {
                    process_invoke(&invoke, depth + 2);
                }
            }
        } else {
            panic!("Unexpected instruction type");
        }
    }
}

fn main() {
    let file_name = std::env::args().nth(1).unwrap_or("txs.bin".to_string());
    let mut file = std::fs::File::open(file_name.clone()).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();

    println!("Data[0] = {}", data[0] as u16);

    let mut txs : Vec<RadixRuntimeFuzzerTransaction> = Vec::new();
    let mut decoder = ScryptoDecoder::new(&data[1..], SCRYPTO_SBOR_V1_MAX_DEPTH);
    while decoder.remaining_bytes() > 0 {
        decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX).map_err(|_| ()).unwrap();
        let instructions = decoder.decode::<Vec<u8>>().map_err(|_| ()).unwrap();
        decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX).map_err(|_| ()).unwrap();
        let invokes = decoder.decode::<Vec<RadixRuntimeFuzzerInput>>().map_err(|_| ()).unwrap();
        
        println!("");

        let instructions : Vec<InstructionV1> = manifest_decode(&instructions).unwrap();
        for instruction in instructions {
            println!("-- {:?}", instruction);
            match instruction {
                InstructionV1::CallFunction { args: ManifestValue::String { value } , .. }
                | InstructionV1::CallMethod { args: ManifestValue::String { value } , .. } => {
                    
                }
                _ => {}
            }
        }
        let invokes = invokes.iter().map(|invoke| {
            invoke.iter().map(|instruction| {
                let instruction: RadixRuntimeFuzzerInstruction = scrypto_decode(&instruction).unwrap();
                println!("{:?}", instruction);
                instruction
            }).collect::<Vec<RadixRuntimeFuzzerInstruction>>()
        }).collect::<Vec<Vec<RadixRuntimeFuzzerInstruction>>>();
    }
    /*
    while decoder.remaining_bytes() > 0 {
        decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX).is_ok();
        let tx = decoder.decode::<RadixRuntimeFuzzerTransaction>();
        if let Ok(tx) = tx {
            txs.push(tx);
        }
    }

    for (tx_id, tx) in txs.iter().enumerate() {
        tx.set_global_invokes();
        println!("TRANSACTION {}", tx_id);
        let instructions : Vec<InstructionV1> = manifest_decode(&tx.instructions).unwrap();
        for instruction in instructions {
            println!("-- {:?}", instruction);
            match instruction {
                InstructionV1::CallFunction { args: ManifestValue::String { value } , .. }
                | InstructionV1::CallMethod { args: ManifestValue::String { value } , .. } => {
                    let invoke = RADIX_RUNTIME_FUZZING_INVOKES.get_invoke(value).unwrap();
                    process_invoke(&invoke, 4);
                }
                _ => {}
            }
        }
    }
     */

}

