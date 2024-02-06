use radix_engine::types::scrypto_decode;
use radix_runtime_fuzzer_common::RadixRuntimeFuzzerTransaction;

use radix_engine_common::prelude::*;
use transaction::model::InstructionV1;
use std::{fs::OpenOptions, io::Write};
use radix_engine::vm::wasm_runtime::RadixRuntimeFuzzerInstruction;
use std::env;

// converts Vec<RadixRuntimeFuzzerTransaction> to seed corpus for fuzzer
fn main() {
    let args: Vec<String> = env::args().collect();
    let input_dir = if args.len() > 1 {
        &args[1]
    } else {
        "../radix-runtime-fuzzer-test-cases/raw"
    };
    let output_dir = if args.len() > 2 {
        &args[2]
    } else {
        "../radix-runtime-fuzzer-test-cases/extracted"
    };

    // read all files in input_dir
    let entries = std::fs::read_dir(input_dir).unwrap();
    let mut txs_vec = Vec::new();
    for entry in entries {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let file_name = file_name.to_str().unwrap();
        let file_name = format!("{}/{}", input_dir, file_name);
        let file_data = std::fs::read(file_name.clone()).unwrap();
        let txs = RadixRuntimeFuzzerTransaction::vec_from_slice(&file_data);
        if txs.is_err() {
            println!("Failed to decode file: {}", file_name);
            continue;
        }
        let txs = txs.unwrap();
        // check if any txs has invokes non empty
        let mut invoke_instructions = 0;
        for tx in &txs {
            for invoke in &tx.invokes {
                invoke_instructions += invoke.len();
            }
        }
        //if invoke_instructions > txs.len() * 19 {
            txs_vec.push(txs);
        //}
    }

    if std::fs::metadata(output_dir).is_err() {
        std::fs::create_dir(output_dir).unwrap();
    }

    let txs_vec_data = scrypto_encode(&txs_vec).unwrap();
    std::fs::write(format!("{}/txs.bin", output_dir), txs_vec_data).unwrap();

    for (tx_id, txs) in txs_vec.iter().enumerate() {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(false)
            .open(format!("{}/{}.bin", output_dir, tx_id))
            .unwrap();
        let mut buf : Vec<u8> = Vec::with_capacity(1024 * 16);
        let mut encoder = ManifestEncoder::new(&mut buf, SCRYPTO_SBOR_V1_MAX_DEPTH);
        encoder.write_byte(tx_id as u8).unwrap();
        for tx in txs {
            let instructions : Vec<InstructionV1> = manifest_decode(&tx.instructions).unwrap();
            for instruction in instructions {
                encoder.encode(&instruction).unwrap();
            }
            encoder.write_byte(0xFF).unwrap();
            for invoke in &tx.invokes {
                for instruction_data in invoke {
                    let instruction : RadixRuntimeFuzzerInstruction = scrypto_decode(&instruction_data).unwrap();
                    encoder.encode(&instruction).unwrap();
                }    
            }
            encoder.write_byte(0xFF).unwrap();
        }
        file.write_all(&buf).unwrap();
    }
}

