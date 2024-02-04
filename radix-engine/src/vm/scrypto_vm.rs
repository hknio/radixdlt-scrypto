use crate::errors::{RuntimeError, SystemUpstreamError, VmError};
use crate::types::*;
use crate::vm::vm::VmInvoke;
use crate::vm::wasm::*;
use crate::vm::wasm_runtime::{ScryptoRuntime, RadixRuntimeFuzzerInstruction};
use crate::vm::VmApi;
use radix_engine_interface::api::ClientApi;
use radix_engine_interface::blueprints::package::CodeHash;
use resources_tracker_macro::trace_resources;

use radix_runtime_fuzzer_common::*;

pub struct ScryptoVm<W: WasmEngine> {
    pub wasm_engine: W,
    pub wasm_validator_config: WasmValidatorConfigV1,
}

impl<W: WasmEngine + Default> Default for ScryptoVm<W> {
    fn default() -> Self {
        Self {
            wasm_engine: W::default(),
            wasm_validator_config: WasmValidatorConfigV1::new(),
        }
    }
}

impl<W: WasmEngine> ScryptoVm<W> {
    pub fn create_instance(
        &self,
        package_address: &PackageAddress,
        code_hash: CodeHash,
        instrumented_code: &[u8],
    ) -> ScryptoVmInstance<W::WasmInstance> {
        ScryptoVmInstance {
            instance: self.wasm_engine.instantiate(code_hash, instrumented_code),
            package_address: *package_address,
        }
    }
}

pub struct ScryptoVmInstance<I: WasmInstance> {
    instance: I,
    package_address: PackageAddress,
}

impl<I: WasmInstance> VmInvoke for ScryptoVmInstance<I> {
    #[trace_resources(log=self.package_address.is_native_package(), log=self.package_address.to_hex(), log=export_name)]
    fn invoke<Y, V>(
        &mut self,
        export_name: &str,
        args: &IndexedScryptoValue,
        api: &mut Y,
        _vm_api: &V,
    ) -> Result<IndexedScryptoValue, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
        V: VmApi,
    {
        radix_runtime_logger!(invoke_start());

        let rtn = {
            let mut runtime: Box<dyn WasmRuntime> = Box::new(ScryptoRuntime::new(
                api,
                self.package_address,
                export_name.to_string(),
            ));

            let mut input = Vec::new();
            input.push(
                runtime
                    .allocate_buffer(args.as_slice().to_vec())
                    .expect("Failed to allocate buffer"),
            );
            self.instance
                .invoke_export(export_name, input, &mut runtime)?
        };

        let output = IndexedScryptoValue::from_vec(rtn).map_err(|e| {
            RuntimeError::SystemUpstreamError(SystemUpstreamError::OutputDecodeError(e))
        })?;

        radix_runtime_logger!(invoke_end(&scrypto_encode(&RadixRuntimeFuzzerInstruction::Return(output.as_vec_ref().clone())).unwrap()));

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    const _: () = {
        fn assert_sync<T: Sync>() {}

        fn assert_all() {
            // The ScryptoInterpreter struct captures the code and module template caches.
            // We therefore share a ScryptoInterpreter as a shared cache across Engine runs on the node.
            // This allows EG multiple mempool submission validations via the Core API at the same time
            // This test ensures the requirement for this cache to be Sync isn't broken
            // (At least when we compile with std, as the node does)
            #[cfg(not(feature = "alloc"))]
            assert_sync::<crate::vm::ScryptoVm<crate::vm::wasm::DefaultWasmEngine>>();
        }
    };
}
