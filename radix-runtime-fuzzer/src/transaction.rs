use transaction::{model::{ExecutionContext, InstructionV1}, prelude::{node_modules::auth::AuthAddresses, Executable}};
use radix_engine_common::prelude::*;

use crate::fuzzer::RadixRuntimeFuzzerInput;

pub const INVOKE_MAGIC_STRING: &str = "INVOKE";
#[cfg(feature="radix_engine_fuzzing")] 
pub static RADIX_RUNTIME_FUZZING_INVOKES : once_cell::sync::Lazy<std::sync::Mutex<Vec<RadixRuntimeFuzzerInput>>> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(Vec::new()));

#[cfg(feature="radix_engine_fuzzing")] 
pub trait RadixRuntimeFuzzingInvokes {
    fn get_invoke_from_slice(&self, data: &[u8]) -> Result<RadixRuntimeFuzzerInput, String> {
        self.get_invoke(scrypto_decode::<String>(data).map_err(|_| String::from("Invalid data"))?)
    }
    fn get_invoke(&self, invoke_string: String) -> Result<RadixRuntimeFuzzerInput, String>;
}

#[cfg(feature="radix_engine_fuzzing")] 
impl RadixRuntimeFuzzingInvokes for once_cell::sync::Lazy<std::sync::Mutex<Vec<RadixRuntimeFuzzerInput>>> {
    fn get_invoke(&self, invoke_string: String) -> Result<RadixRuntimeFuzzerInput, String> {
        if !invoke_string.starts_with(INVOKE_MAGIC_STRING) {
            return Err(String::from("Invalid invoke string"));
        }
        let invoke_id_string = invoke_string.replace(&format!("{}_", INVOKE_MAGIC_STRING), "");
        let invoke_id = invoke_id_string.parse::<usize>().map_err(|_| String::from("Invalid invoke, not a number"))?;
        let fuzz_data = self.lock().unwrap().get(invoke_id).ok_or(String::from("Invalid invoke number"))?.clone();
        Ok(fuzz_data)
    }
}

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

    #[cfg(feature="radix_engine_fuzzing")]
    pub fn set_global_invokes(&self) {
        let mut invokes = RADIX_RUNTIME_FUZZING_INVOKES.lock().unwrap();
        invokes.clear();
        invokes.extend(self.invokes.clone());
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

