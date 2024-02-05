mod no_op_runtime;
mod scrypto_runtime;

pub use no_op_runtime::NoOpWasmRuntime;
pub use scrypto_runtime::ScryptoRuntime;

#[cfg(any(feature = "radix_runtime_logger", feature = "radix_runtime_fuzzing"))]
pub use scrypto_runtime::RadixRuntimeFuzzerInstruction;
