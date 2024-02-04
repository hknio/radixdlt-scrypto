
pub type RadixRuntimeFuzzerInput = Vec<Vec<u8>>;
pub trait RadixRuntimeFuzzer {
    fn execute_instructions(&mut self, instructions: RadixRuntimeFuzzerInput) -> Result<Vec<u8>, ()>;
}
