use std::{fs::OpenOptions, io::Write};
use transaction::{model::{ExecutionContext, InstructionV1}, prelude::{node_modules::auth::AuthAddresses, Executable}};
use radix_engine_common::prelude::*;

use crate::fuzzer::RadixRuntimeFuzzerInput;
use crate::transaction::RadixRuntimeFuzzerTransaction;

pub struct RadixRuntimeInvokeLogger {
    instructions: Vec<Vec<u8>>,
    depth: usize,
}

impl RadixRuntimeInvokeLogger {
    pub fn new(data : &Vec<u8>) -> Self {
        Self {
            instructions: Vec::new(),
            depth: 0,
        }
    }

    pub fn finish(&mut self, data : &Vec<u8>) -> &RadixRuntimeFuzzerInput {
        self.instructions.push(data.clone());
        &self.instructions
    }

    pub fn runtime_call_start(&mut self, func_name : String, data: Vec<u8>) {
        if self.depth == 0 {
            self.instructions.push(data);
        }
        self.depth += 1;
    }

    pub fn invoke_in_invoke_end(&mut self, mut invoke_logger : RadixRuntimeInvokeLogger, data: &Vec<u8>) {
        // replace last arguments of last instruction with invoke_logger.finish(data)
        // let arguments = &mut self.instructions.last_mut().unwrap();
        // arguments.pop();
        // arguments.push(scrypto_encode(&scrypto_encode(&invoke_logger.finish(data)).unwrap()).unwrap());
    }

    pub fn runtime_call_end(&mut self, func_name : String, data: Option<Vec<u8>>) {
        self.depth -= 1;
    }
}

pub struct RadixRuntimeLogger {
    instructions : Vec<InstructionV1>,
    references: IndexSet<Reference>,
    blobs: IndexMap<Hash, Vec<u8>>,
    execution_context: Option<ExecutionContext>,
    instruction_index: usize,
    invoke_loggers: Vec<RadixRuntimeInvokeLogger>,
    invokes: Vec<RadixRuntimeFuzzerInput>,
    tx_id: usize
}

impl RadixRuntimeLogger {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            references: index_set_new(),
            blobs: index_map_new(),
            execution_context: None,
            instruction_index: 0,
            invoke_loggers: Vec::new(),
            invokes: Vec::new(),
            tx_id: 0
        }
    }

    fn write_to_file(&self, data: Vec<u8>) {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(false)
            .open(format!("tx_{}.bin", self.tx_id))
            .unwrap();
        file.write_all(&data).unwrap();
    }

    pub fn transaction_execution_start(&mut self, executable: &Executable) {
        self.instructions = manifest_decode::<Vec<InstructionV1>>(&executable.encoded_instructions()).unwrap().clone();
        self.references = executable.references().clone();
        self.blobs = executable.blobs().clone();
        self.execution_context = Some(executable.context().clone());
        self.instruction_index = 0;
        self.invoke_loggers = Vec::new();
        self.invokes = Vec::new();
    }

    pub fn transaction_execution_end(&mut self, success: bool) {
        assert!(self.instruction_index == self.instructions.len()); // just in case

        if self.execution_context.as_ref().unwrap().auth_zone_params.initial_proofs == btreeset!(AuthAddresses::system_role()) {
            return; // system transaction
        }

        let data = RadixRuntimeFuzzerTransaction {
            instructions: manifest_encode(&self.instructions).unwrap(),
            references: self.references.clone(),
            blobs: self.blobs.clone(),
            execution_context: self.execution_context.clone().unwrap(),
            invokes: self.invokes.clone(),
        };
        self.write_to_file(scrypto_encode(&data).unwrap());
        self.tx_id += 1;
    }

    pub fn instruction_start(&mut self, instruction: &InstructionV1) {
        assert!(self.instructions[self.instruction_index] == *instruction);
        self.instruction_index += 1;
    }

    pub fn invoke_start(&mut self, data: &Vec<u8>) {       
        self.invoke_loggers.push(RadixRuntimeInvokeLogger::new(data));
    }

    pub fn invoke_end(&mut self, data: &Vec<u8>) {
        let mut invoke_logger = self.invoke_loggers.pop().unwrap();
        if self.invoke_loggers.len() > 0 {
            self.invoke_loggers.last_mut().unwrap().invoke_in_invoke_end(invoke_logger, data);
        } else {
            let invoke_instructions = invoke_logger.finish(data);
            self.invokes.push(invoke_instructions.clone());
            let instruction = &mut self.instructions[self.instruction_index - 1];
            match instruction {
                InstructionV1::CallFunction { args, .. }
                | InstructionV1::CallMethod { args, .. } => {
                    *args = ManifestValue::String { value: "fuzz_invoke".to_string() };
                }
                _ => {
                    panic!("Unexpected instruction type");
                }
            }
        }
    }

    pub fn runtime_call_start(&mut self, func_name : String, data: Vec<u8>) {
        self.invoke_loggers.last_mut().unwrap().runtime_call_start(func_name, data);
    }

    pub fn runtime_call_end(&mut self, func_name : String, data: Option<Vec<u8>>) {
        self.invoke_loggers.last_mut().unwrap().runtime_call_end(func_name, data);
    }
}

#[cfg(feature="runtime_logger")] 
pub static RADIX_RUNTIME_LOGGER: once_cell::sync::Lazy<std::sync::Mutex<RadixRuntimeLogger>> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(RadixRuntimeLogger::new()));

#[cfg(feature="runtime_logger")]
#[macro_export]
macro_rules! radix_runtime_logger {
    ($($arg:tt)*) => {
        $crate::RADIX_RUNTIME_LOGGER.lock().unwrap().$($arg)*
    };
}

#[cfg(not(feature="runtime_logger"))]
#[macro_export]
macro_rules! radix_runtime_logger {
    ($($arg:tt)*) => {};
}
