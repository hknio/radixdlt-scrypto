use core::panic;
use std::{fs::OpenOptions, io::Write};
use transaction::{model::{ExecutionContext, InstructionV1}, prelude::{node_modules::auth::AuthAddresses, Executable}};
use radix_engine_common::prelude::*;

use crate::fuzzer::RadixRuntimeFuzzerInput;
use crate::transaction::RadixRuntimeFuzzerTransaction;

pub struct RadixRuntimeInvokeLogger {
    instructions: Vec<Vec<u8>>,
    first_instruction: bool,
    depth: usize,
}

impl RadixRuntimeInvokeLogger {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            first_instruction: true,
            depth: 0,
        }
    }

    pub fn instructions(&self) -> &RadixRuntimeFuzzerInput {
        &self.instructions
    }

    pub fn finish(&mut self, data : &Vec<u8>) {
        self.instructions.push(data.clone());
    }

    pub fn runtime_call_start(&mut self, data: Vec<u8>) {
        if self.depth == 0 {
            if self.first_instruction {
                self.first_instruction = false; // first instruction is allocate_buffer with args, skip it
            } else {
                self.instructions.push(data);
            }
        }
        self.depth += 1;
    }

    pub fn runtime_call_end(&mut self, data: Option<Vec<u8>>) {
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
    invoke_index: Vec<usize>,
    tx_id: usize,
    write: bool
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
            invoke_index: Vec::new(),
            tx_id: 0,
            write: true
        }
    }

    fn get_file_name(&self) -> String {
        std::env::var("RADIX_RUNTIME_LOGGER_FILE_NAME").unwrap_or("txs.bin".to_string())
    }

    fn write_to_file(&self, data: Vec<u8>) {
        if !self.write {
            return;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(self.get_file_name())
            .unwrap();
        file.write_all(&data).unwrap();
    }

    fn current_invoke_logger(&mut self) -> &mut RadixRuntimeInvokeLogger {
        &mut self.invoke_loggers[*self.invoke_index.last().unwrap()]
    }

    pub fn disable_write_to_file(&mut self) {
        self.write = false;
    }

    pub fn transaction_execution_start(&mut self, executable: &Executable) {
        self.instructions = manifest_decode::<Vec<InstructionV1>>(&executable.encoded_instructions()).unwrap().clone();
        self.references = executable.references().clone();
        self.blobs = index_map_new();
        self.execution_context = Some(executable.context().clone());
        self.instruction_index = 0;
        self.invoke_loggers = Vec::new();
        self.invoke_index = Vec::new();

        // change blobs to empty, we don't need them
        for (hash, _) in executable.blobs() {
            self.blobs.insert(hash.clone(), Vec::new());
        }
    }

    pub fn transaction_execution_end(&mut self, success: bool) {
        assert!(!success || self.instruction_index == self.instructions.len());
        if self.execution_context.as_ref().unwrap().auth_zone_params.initial_proofs == btreeset!(AuthAddresses::system_role()) {
            return; // skip system transaction
        }
        if !success {
            return; // skip failed transactions
        }

        if self.tx_id == 0 {
            // remove file with transactiosn if it already exists
            if std::fs::metadata(self.get_file_name()).is_ok() {
                std::fs::remove_file(self.get_file_name()).unwrap();
            }
        }

        let data = RadixRuntimeFuzzerTransaction {
            instructions: manifest_encode(&self.instructions).unwrap(),
            references: self.references.clone(),
            blobs: self.blobs.clone(),
            execution_context: self.execution_context.clone().unwrap(),
            invokes: self.invoke_loggers.iter().map(|logger| logger.instructions().clone()).collect()
        };
        self.write_to_file(scrypto_encode(&data).unwrap());
        self.tx_id += 1;
    }

    pub fn instruction_start(&mut self, instruction: &InstructionV1) {
        assert!(self.instructions[self.instruction_index] == *instruction);
        self.instruction_index += 1;
    }

    pub fn invoke_start(&mut self, data: &Vec<u8>) {       
        self.invoke_index.push(self.invoke_loggers.len());
        self.invoke_loggers.push(RadixRuntimeInvokeLogger::new());
    }

    pub fn invoke_end(&mut self, data: &Vec<u8>) {
        let index = self.invoke_index.pop().unwrap();
        let invoke_logger = &mut self.invoke_loggers[index];
        invoke_logger.finish(data);
    }

    pub fn runtime_call_start(&mut self, data: Vec<u8>) {
        self.current_invoke_logger().runtime_call_start(data);
    }

    pub fn runtime_call_end(&mut self, data: Option<Vec<u8>>) {
        self.current_invoke_logger().runtime_call_end(data);
    }
}

#[cfg(feature="radix_runtime_logger")] 
pub static RADIX_RUNTIME_LOGGER: once_cell::sync::Lazy<std::sync::Mutex<RadixRuntimeLogger>> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(RadixRuntimeLogger::new()));

#[cfg(feature="radix_runtime_logger")]
#[macro_export]
macro_rules! radix_runtime_logger {
    ($($arg:tt)*) => {
        $crate::RADIX_RUNTIME_LOGGER.lock().unwrap().$($arg)*
    };
}
