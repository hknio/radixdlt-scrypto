use std::{fs::OpenOptions, io::Write};

use once_cell::sync::Lazy;
use transaction::{model::{ExecutionContext, InstructionV1}, prelude::{node_modules::auth::AuthAddresses, Executable}};
use radix_engine_common::{data::scrypto, prelude::*};

pub struct RadixRuntimeInvokeLogger {
    instructions: Vec<(u8, Vec<Vec<u8>>)>,
    depth: usize,
}

impl RadixRuntimeInvokeLogger {
    pub fn new(data : &Vec<u8>) -> Self {
        Self {
            instructions: Vec::new(),
            depth: 0,
        }
    }

    pub fn finish(&mut self, data : &Vec<u8>) -> &Vec<(u8, Vec<Vec<u8>>)> {
        self.instructions.push((0xFF, vec![data.clone()]));
        &self.instructions
    }

    pub fn runtime_call_start(&mut self, func_name : String, func_id: u8, data: Vec<Vec<u8>>) {
        if self.depth == 0 {
            self.instructions.push((func_id, data));
        }
        self.depth += 1;
    }

    pub fn invoke_in_invoke_end(&mut self, mut invoke_logger : RadixRuntimeInvokeLogger, data: &Vec<u8>) {
        // replace last arguments of last instruction with invoke_logger.finish(data)
        let arguments = &mut self.instructions.last_mut().unwrap().1;
        arguments.pop();
        arguments.push(scrypto_encode(&scrypto_encode(&invoke_logger.finish(data)).unwrap()).unwrap());
    }

    pub fn runtime_call_end(&mut self, func_name : String, func_id: u8, data: Vec<Vec<u8>>) {
        self.depth -= 1;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub struct RadixRuntimeFuzzerData {
    instructions : Vec<u8>,
    references: IndexSet<Reference>,
    blobs: IndexMap<Hash, Vec<u8>>,
    execution_context: ExecutionContext,
}

impl RadixRuntimeFuzzerData {
    pub fn get_executable<'a>(
        &'a self,
    ) -> Executable<'a> {
        Executable::new(
            &self.instructions,
            &self.references,
            &self.blobs,
            self.execution_context.clone(),
        )
    }
}

pub struct RadixRuntimeLogger {
    instructions : Vec<InstructionV1>,
    references: IndexSet<Reference>,
    blobs: IndexMap<Hash, Vec<u8>>,
    execution_context: Option<ExecutionContext>,
    instruction_index: usize,
    invoke_loggers: Vec<RadixRuntimeInvokeLogger>,
    tx_id: usize,
    enabled: bool
}

impl RadixRuntimeLogger {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            references: IndexSet::new(),
            blobs: IndexMap::new(),
            execution_context: None,
            instruction_index: 0,
            invoke_loggers: Vec::new(),
            tx_id: 0,
            enabled: true,
        }
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    fn write_line_to_file(&self, line: String) {
        if !self.enabled {
            return;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open("log.txt")
            .unwrap();
        file.write_all(format!("{}\n", line).as_bytes()).unwrap();
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
        if !self.enabled {
            return;
        }

        self.instructions = manifest_decode::<Vec<InstructionV1>>(&executable.encoded_instructions()).unwrap().clone();
        self.references = executable.references().clone();
        self.blobs = executable.blobs().clone();
        self.execution_context = Some(executable.context().clone());
        self.instruction_index = 0;
        self.invoke_loggers = Vec::new();
    }

    pub fn transaction_execution_end(&mut self, success: bool) {
        if !success || !self.enabled {
            return;        
        }
        assert!(self.instruction_index == self.instructions.len()); // just in case

        if self.execution_context.as_ref().unwrap().auth_zone_params.initial_proofs == btreeset!(AuthAddresses::system_role()) {
            return; // system transaction
        }

        let data = RadixRuntimeFuzzerData {
            instructions: manifest_encode(&self.instructions).unwrap(),
            references: self.references.clone(),
            blobs: self.blobs.clone(),
            execution_context: self.execution_context.clone().unwrap(),
        };
        self.write_to_file(scrypto_encode(&data).unwrap());
        self.tx_id += 1;
    }

    pub fn instruction_start(&mut self, instruction: &InstructionV1) {
        if !self.enabled {
            return;
        }

        assert!(self.instructions[self.instruction_index] == *instruction);
        self.instruction_index += 1;
    }

    pub fn invoke_start(&mut self, data: &Vec<u8>) {
        if !self.enabled {
            return;
        }
        
        self.invoke_loggers.push(RadixRuntimeInvokeLogger::new(data));
    }

    pub fn invoke_end(&mut self, data: &Vec<u8>) {
        if !self.enabled {
            return;
        }

        let mut invoke_logger = self.invoke_loggers.pop().unwrap();
        if self.invoke_loggers.len() > 0 {
            self.invoke_loggers.last_mut().unwrap().invoke_in_invoke_end(invoke_logger, data);
        } else {
            let invoke_instructions = &invoke_logger.finish(data);
            let instruction = &mut self.instructions[self.instruction_index - 1];
            match instruction {
                InstructionV1::CallFunction { args, .. }
                | InstructionV1::CallMethod { args, .. } => {
                    *args = to_manifest_value(invoke_instructions).unwrap()
                }
                _ => {
                    panic!("Unexpected instruction type");
                }
            }
        }
    }

    pub fn runtime_call_start(&mut self, func_name : String, func_id: u8, data: Vec<Vec<u8>>) {
        if !self.enabled {
            return;
        }
        self.invoke_loggers.last_mut().unwrap().runtime_call_start(func_name, func_id, data);
    }

    pub fn runtime_call_end(&mut self, func_name : String, func_id: u8, data: Vec<Vec<u8>>) {
        if !self.enabled {
            return;
        }
        self.invoke_loggers.last_mut().unwrap().runtime_call_end(func_name, func_id, data);
    }
}

pub static RADIX_RUNTIME_LOGGER: Lazy<std::sync::Mutex<RadixRuntimeLogger>> = Lazy::new(|| std::sync::Mutex::new(RadixRuntimeLogger::new()));
                
pub trait RadixRuntimeFuzzer {
    fn execute_func(&mut self, func_id: u8, fuzz_args: Vec<Vec<u8>>) -> Result<(), ()>;
    fn fuzz(&mut self, instructions : Vec<(u8, Vec<Vec<u8>>)>) -> Result<Vec<u8>, ()> {
        for (func_id, mut args) in instructions {
            if func_id == 0xFF {
                return Ok(args.pop().unwrap());
            }
            self.execute_func(func_id, args).unwrap();
        }

        Err(())
    }
}