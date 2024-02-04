use crate::errors::InvokeError;
use crate::types::*;
use crate::vm::wasm::errors::*;
use crate::vm::wasm::traits::*;
use radix_engine_interface::blueprints::package::CodeHash;
use lazy_static::lazy_static;
use radix_runtime_fuzzer_common::RadixRuntimeFuzzerInput;
use std::sync::Mutex;

lazy_static! {
    static ref FUZZER_ENGINE_INVOKES: Mutex<Vec<RadixRuntimeFuzzerInput>> = Mutex::new(Vec::new());
}

pub struct FuzzerInstance {
    code: Option<RadixRuntimeFuzzerInput>
}

impl FuzzerInstance {
    pub fn new(code: Option<RadixRuntimeFuzzerInput>) -> Self {
        Self {
            code
        }
    }
}

impl WasmInstance for FuzzerInstance {
    fn invoke_export<'r>(
        &mut self,
        func_name: &str,
        _args: Vec<Buffer>,
        runtime: &mut Box<dyn WasmRuntime + 'r>,
    ) -> Result<Vec<u8>, InvokeError<WasmRuntimeError>> {
        if self.code.is_none() {
            return Err(InvokeError::SelfError(WasmRuntimeError::ExecutionError(format!("No code to execute in {}", func_name).to_string())));
        }
        runtime.execute_instructions(self.code.as_ref().unwrap()).map_err(|_| {
            InvokeError::SelfError(WasmRuntimeError::ExecutionError(format!("Fuzzing error in {}", func_name).to_string()))
        })
    }
}


pub struct FuzzerEngine {}

impl Default for FuzzerEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FuzzerEngine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn set_invokes(new_invokes : Vec<RadixRuntimeFuzzerInput>) {
        let mut invokes = FUZZER_ENGINE_INVOKES.lock().unwrap();
        invokes.clear();
        invokes.extend(new_invokes.iter().rev().cloned());
    }

    pub fn get_invoke() -> Option<RadixRuntimeFuzzerInput> {
        FUZZER_ENGINE_INVOKES.lock().unwrap().pop()
    }
}

impl WasmEngine for FuzzerEngine {
    type WasmInstance = FuzzerInstance;

    fn instantiate(&self, _code_hash: CodeHash, _instrumented_code: &[u8]) -> FuzzerInstance {
        FuzzerInstance::new(
            FuzzerEngine::get_invoke()
        )
    }
}
