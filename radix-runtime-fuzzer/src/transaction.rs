use transaction::{model::{ExecutionContext, InstructionV1}, prelude::{node_modules::auth::AuthAddresses, Executable}};
use radix_engine_common::prelude::*;

use crate::fuzzer::RadixRuntimeFuzzerInput;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct RadixRuntimeFuzzerTransaction {
    pub instructions : Vec<u8>,
    pub references: IndexSet<Reference>,
    pub blobs: IndexMap<Hash, Vec<u8>>,
    pub execution_context: ExecutionContext,
    pub invokes: Vec<RadixRuntimeFuzzerInput>,
}

impl RadixRuntimeFuzzerTransaction {
    pub fn get_executable<'a>(
        &'a mut self,
    ) -> Executable<'a> {
        let mut invoke_id = 0;
        let mut instructions = manifest_decode::<Vec<InstructionV1>>(&self.instructions).unwrap().clone();        
        for instruction in &mut instructions {
            match instruction {
                InstructionV1::CallFunction { args, .. }
                | InstructionV1::CallMethod { args, .. } => {
                    match args {
                        ManifestValue::String { value } => {
                            if value.starts_with("fuzz_invoke") {
                                *args = to_manifest_value(&self.invokes[invoke_id]).unwrap();
                                invoke_id += 1;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        self.instructions = manifest_encode(&instructions).unwrap();

        Executable::new(
            &self.instructions,
            &self.references,
            &self.blobs,
            self.execution_context.clone(),
        )
    }
}
