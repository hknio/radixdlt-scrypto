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
    pub fn vec_from_slice(data: &[u8]) -> Result<Vec<RadixRuntimeFuzzerTransaction>, DecodeError> {
        let mut txs : Vec<RadixRuntimeFuzzerTransaction> = Vec::new();
        let mut decoder = ScryptoDecoder::new(data, SCRYPTO_SBOR_V1_MAX_DEPTH);
        while decoder.remaining_bytes() > 0 {
            decoder.read_and_check_payload_prefix(SCRYPTO_SBOR_V1_PAYLOAD_PREFIX)?;
            let tx = decoder.decode::<RadixRuntimeFuzzerTransaction>()?;
            txs.push(tx);
        }
        Ok(txs)
    }

    pub fn get_executable<'a>(
        &'a mut self,
    ) -> Executable<'a> {
        Executable::new(
            &self.instructions,
            &self.references,
            &self.blobs,
            self.execution_context.clone(),
        )
    }
}

