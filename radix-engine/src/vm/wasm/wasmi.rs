extern crate radix_wasmi as wasmi;

use crate::errors::InvokeError;
use crate::internal_prelude::*;
#[cfg(feature = "coverage")]
use crate::utils::save_coverage_data;
use crate::vm::wasm::constants::*;
use crate::vm::wasm::errors::*;
use crate::vm::wasm::traits::*;
use crate::vm::wasm::WasmEngine;
use radix_engine_interface::api::actor_api::EventFlags;
use radix_engine_interface::blueprints::package::CodeHash;
use sbor::rust::mem::transmute;
use sbor::rust::mem::MaybeUninit;
#[cfg(not(feature = "fuzzing"))]
use sbor::rust::sync::Arc;
use wasmi::core::Value;
use wasmi::core::{HostError, Trap};
use wasmi::errors::InstantiationError;
use wasmi::*;

type FakeHostState = FakeWasmiInstanceEnv;
type HostState = WasmiInstanceEnv;

/// A `WasmiModule` defines a parsed WASM module "template" Instance (with imports already defined)
/// and Store, which keeps user data.
/// "Template" (Store, Instance) tuple are cached together, and never to be invoked.
/// Upon instantiation Instance and Store are cloned, so the state is not shared between instances.
/// It is safe to clone an `Instance` and a `Store`, since they don't use pointers, but `Arena`
/// allocator. `Instance` is owned by `Store`, it is basically some offset within `Store`'s vector
/// of `Instance`s. So after clone we receive the same `Store`, where we are able to set different
/// data, more specifically a `runtime_ptr`.
/// Also, it is correctly `Send + Sync` (under the assumption that the data in the Store is set to
/// a valid value upon invocation , because this is the thing which is cached in the
/// ScryptoInterpreter caches.
pub struct WasmiModule {
    template_store: Store<FakeHostState>,
    template_instance: Instance,
    #[allow(dead_code)]
    code_size_bytes: usize,
}

pub struct WasmiInstance {
    store: Store<HostState>,
    instance: Instance,
    memory: Memory,
}

/// This is to construct a stub `Store<FakeWasmiInstanceEnv>`, which is a part of
/// `WasmiModule` struct and serves as a placeholder for the real `Store<WasmiInstanceEnv>`.
/// The real store is set (prior being transumted) when the `WasmiModule` is being instantiated.
/// In fact the only difference between a stub and real Store is the `Send + Sync` manually
/// implemented for the former one, which is required by `WasmiModule` cache (for `std`
/// configuration) but shall not be implemented for the latter one to prevent sharing it between
/// the threads since pointer might point to volatile data.
#[derive(Clone)]
pub struct FakeWasmiInstanceEnv {
    #[allow(dead_code)]
    runtime_ptr: MaybeUninit<*mut Box<dyn WasmRuntime>>,
}

impl FakeWasmiInstanceEnv {
    pub fn new() -> Self {
        Self {
            runtime_ptr: MaybeUninit::uninit(),
        }
    }
}

unsafe impl Send for FakeWasmiInstanceEnv {}
unsafe impl Sync for FakeWasmiInstanceEnv {}

/// This is to construct a real `Store<WasmiInstanceEnv>
pub struct WasmiInstanceEnv {
    runtime_ptr: MaybeUninit<*mut Box<dyn WasmRuntime>>,
}

impl WasmiInstanceEnv {
    pub fn new() -> Self {
        Self {
            runtime_ptr: MaybeUninit::uninit(),
        }
    }
}

macro_rules! grab_runtime {
    ($caller: expr) => {{
        let runtime: &mut Box<dyn WasmRuntime> =
            unsafe { &mut *$caller.data().runtime_ptr.assume_init() };
        let memory = match $caller.get_export(EXPORT_MEMORY) {
            Some(Extern::Memory(memory)) => memory,
            _ => panic!("Failed to find memory export"),
        };
        (memory, runtime)
    }};
}

// native functions start
fn consume_buffer(
    caller: Caller<'_, HostState>,
    buffer_id: BufferId,
    destination_ptr: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let result = runtime.buffer_consume(buffer_id);
    match result {
        Ok(slice) => {
            write_memory(caller, memory, destination_ptr, &slice)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn call_method(
    mut caller: Caller<'_, HostState>,
    receiver_ptr: u32,
    receiver_len: u32,
    ident_ptr: u32,
    ident_len: u32,
    args_ptr: u32,
    args_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let receiver = read_memory(caller.as_context_mut(), memory, receiver_ptr, receiver_len)?;
    let ident = read_memory(caller.as_context_mut(), memory, ident_ptr, ident_len)?;
    let args = read_memory(caller.as_context_mut(), memory, args_ptr, args_len)?;

    runtime
        .object_call(receiver, ident, args)
        .map(|buffer| buffer.0)
}

fn call_direct_method(
    mut caller: Caller<'_, HostState>,
    receiver_ptr: u32,
    receiver_len: u32,
    ident_ptr: u32,
    ident_len: u32,
    args_ptr: u32,
    args_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let receiver = read_memory(caller.as_context_mut(), memory, receiver_ptr, receiver_len)?;
    let ident = read_memory(caller.as_context_mut(), memory, ident_ptr, ident_len)?;
    let args = read_memory(caller.as_context_mut(), memory, args_ptr, args_len)?;

    runtime
        .object_call_direct(receiver, ident, args)
        .map(|buffer| buffer.0)
}

fn call_module_method(
    mut caller: Caller<'_, HostState>,
    receiver_ptr: u32,
    receiver_len: u32,
    module_id: u32,
    ident_ptr: u32,
    ident_len: u32,
    args_ptr: u32,
    args_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let receiver = read_memory(caller.as_context_mut(), memory, receiver_ptr, receiver_len)?;
    let ident = read_memory(caller.as_context_mut(), memory, ident_ptr, ident_len)?;
    let args = read_memory(caller.as_context_mut(), memory, args_ptr, args_len)?;

    runtime
        .object_call_module(receiver, module_id, ident, args)
        .map(|buffer| buffer.0)
}

fn call_function(
    mut caller: Caller<'_, HostState>,
    package_address_ptr: u32,
    package_address_len: u32,
    blueprint_name_ptr: u32,
    blueprint_name_len: u32,
    ident_ptr: u32,
    ident_len: u32,
    args_ptr: u32,
    args_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let package_address = read_memory(
        caller.as_context_mut(),
        memory,
        package_address_ptr,
        package_address_len,
    )?;
    let blueprint_name = read_memory(
        caller.as_context_mut(),
        memory,
        blueprint_name_ptr,
        blueprint_name_len,
    )?;
    let ident = read_memory(caller.as_context_mut(), memory, ident_ptr, ident_len)?;
    let args = read_memory(caller.as_context_mut(), memory, args_ptr, args_len)?;

    runtime
        .blueprint_call(package_address, blueprint_name, ident, args)
        .map(|buffer| buffer.0)
}

fn new_object(
    mut caller: Caller<'_, HostState>,
    blueprint_name_ptr: u32,
    blueprint_name_len: u32,
    object_states_ptr: u32,
    object_states_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    runtime
        .object_new(
            read_memory(
                caller.as_context_mut(),
                memory,
                blueprint_name_ptr,
                blueprint_name_len,
            )?,
            read_memory(
                caller.as_context_mut(),
                memory,
                object_states_ptr,
                object_states_len,
            )?,
        )
        .map(|buffer| buffer.0)
}

fn new_key_value_store(
    mut caller: Caller<'_, HostState>,
    schema_id_ptr: u32,
    schema_id_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    runtime
        .key_value_store_new(read_memory(
            caller.as_context_mut(),
            memory,
            schema_id_ptr,
            schema_id_len,
        )?)
        .map(|buffer| buffer.0)
}

fn allocate_global_address(
    mut caller: Caller<'_, HostState>,
    package_address_ptr: u32,
    package_address_len: u32,
    blueprint_name_ptr: u32,
    blueprint_name_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    runtime
        .address_allocate(
            read_memory(
                caller.as_context_mut(),
                memory,
                package_address_ptr,
                package_address_len,
            )?,
            read_memory(
                caller.as_context_mut(),
                memory,
                blueprint_name_ptr,
                blueprint_name_len,
            )?,
        )
        .map(|buffer| buffer.0)
}

fn get_reservation_address(
    mut caller: Caller<'_, HostState>,
    node_id_ptr: u32,
    node_id_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    runtime
        .address_get_reservation_address(read_memory(
            caller.as_context_mut(),
            memory,
            node_id_ptr,
            node_id_len,
        )?)
        .map(|buffer| buffer.0)
}

fn execution_cost_unit_limit(
    caller: Caller<'_, HostState>,
) -> Result<u32, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.costing_get_execution_cost_unit_limit()
}

fn execution_cost_unit_price(
    caller: Caller<'_, HostState>,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime
        .costing_get_execution_cost_unit_price()
        .map(|buffer| buffer.0)
}

fn finalization_cost_unit_limit(
    caller: Caller<'_, HostState>,
) -> Result<u32, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.costing_get_finalization_cost_unit_limit()
}

fn finalization_cost_unit_price(
    caller: Caller<'_, HostState>,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime
        .costing_get_finalization_cost_unit_price()
        .map(|buffer| buffer.0)
}

fn usd_price(caller: Caller<'_, HostState>) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.costing_get_usd_price().map(|buffer| buffer.0)
}

fn tip_percentage(caller: Caller<'_, HostState>) -> Result<u32, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.costing_get_tip_percentage()
}

fn fee_balance(caller: Caller<'_, HostState>) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.costing_get_fee_balance().map(|buffer| buffer.0)
}

fn globalize_object(
    mut caller: Caller<'_, HostState>,
    obj_id_ptr: u32,
    obj_id_len: u32,
    modules_ptr: u32,
    modules_len: u32,
    address_ptr: u32,
    address_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    runtime
        .globalize_object(
            read_memory(caller.as_context_mut(), memory, obj_id_ptr, obj_id_len)?,
            read_memory(caller.as_context_mut(), memory, modules_ptr, modules_len)?,
            read_memory(caller.as_context_mut(), memory, address_ptr, address_len)?,
        )
        .map(|buffer| buffer.0)
}

fn instance_of(
    mut caller: Caller<'_, HostState>,
    component_id_ptr: u32,
    component_id_len: u32,
    package_address_ptr: u32,
    package_address_len: u32,
    blueprint_name_ptr: u32,
    blueprint_name_len: u32,
) -> Result<u32, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    runtime.instance_of(
        read_memory(
            caller.as_context_mut(),
            memory,
            component_id_ptr,
            component_id_len,
        )?,
        read_memory(
            caller.as_context_mut(),
            memory,
            package_address_ptr,
            package_address_len,
        )?,
        read_memory(
            caller.as_context_mut(),
            memory,
            blueprint_name_ptr,
            blueprint_name_len,
        )?,
    )
}

fn blueprint_id(
    mut caller: Caller<'_, HostState>,
    component_id_ptr: u32,
    component_id_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    runtime
        .blueprint_id(read_memory(
            caller.as_context_mut(),
            memory,
            component_id_ptr,
            component_id_len,
        )?)
        .map(|buffer| buffer.0)
}

fn get_outer_object(
    mut caller: Caller<'_, HostState>,
    component_id_ptr: u32,
    component_id_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    runtime
        .get_outer_object(read_memory(
            caller.as_context_mut(),
            memory,
            component_id_ptr,
            component_id_len,
        )?)
        .map(|buffer| buffer.0)
}

fn lock_key_value_store_entry(
    mut caller: Caller<'_, HostState>,
    node_id_ptr: u32,
    node_id_len: u32,
    offset_ptr: u32,
    offset_len: u32,
    flags: u32,
) -> Result<u32, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let node_id = read_memory(caller.as_context_mut(), memory, node_id_ptr, node_id_len)?;
    let substate_key = read_memory(caller.as_context_mut(), memory, offset_ptr, offset_len)?;

    runtime.key_value_store_open_entry(node_id, substate_key, flags)
}

fn key_value_entry_get(
    caller: Caller<'_, HostState>,
    handle: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);
    runtime.key_value_entry_get(handle).map(|buffer| buffer.0)
}

fn key_value_entry_set(
    mut caller: Caller<'_, HostState>,
    handle: u32,
    buffer_ptr: u32,
    buffer_len: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);
    let data = read_memory(caller.as_context_mut(), memory, buffer_ptr, buffer_len)?;
    runtime.key_value_entry_set(handle, data)
}

fn key_value_entry_remove(
    caller: Caller<'_, HostState>,
    handle: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);
    runtime
        .key_value_entry_remove(handle)
        .map(|buffer| buffer.0)
}

fn unlock_key_value_entry(
    caller: Caller<'_, HostState>,
    handle: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);
    runtime.key_value_entry_close(handle)
}

fn key_value_store_remove(
    mut caller: Caller<'_, HostState>,
    node_id_ptr: u32,
    node_id_len: u32,
    key_ptr: u32,
    key_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);
    let node_id = read_memory(caller.as_context_mut(), memory, node_id_ptr, node_id_len)?;
    let key = read_memory(caller.as_context_mut(), memory, key_ptr, key_len)?;

    runtime
        .key_value_store_remove_entry(node_id, key)
        .map(|buffer| buffer.0)
}

fn lock_field(
    caller: Caller<'_, HostState>,
    object_handle: u32,
    field: u32,
    flags: u32,
) -> Result<u32, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);
    runtime.actor_open_field(object_handle, field as u8, flags)
}

fn field_lock_read(
    caller: Caller<'_, HostState>,
    handle: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.field_entry_read(handle).map(|buffer| buffer.0)
}

fn field_lock_write(
    mut caller: Caller<'_, HostState>,
    handle: u32,
    data_ptr: u32,
    data_len: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let data = read_memory(caller.as_context_mut(), memory, data_ptr, data_len)?;

    runtime.field_entry_write(handle, data)
}

fn field_lock_release(
    caller: Caller<'_, HostState>,
    handle: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.field_entry_close(handle)
}

fn actor_get_node_id(
    caller: Caller<'_, HostState>,
    handle: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.actor_get_node_id(handle).map(|buffer| buffer.0)
}

fn get_package_address(
    caller: Caller<'_, HostState>,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.actor_get_package_address().map(|buffer| buffer.0)
}

fn get_blueprint_name(caller: Caller<'_, HostState>) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    runtime.actor_get_blueprint_name().map(|buffer| buffer.0)
}

fn consume_wasm_execution_units(
    caller: Caller<'_, HostState>,
    n: u64,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let (_memory, runtime) = grab_runtime!(caller);

    // TODO: wasm-instrument uses u64 for cost units. We need to decide if we want to move from u32
    // to u64 as well.
    runtime.consume_wasm_execution_units(n as u32)
}

fn emit_event(
    mut caller: Caller<'_, HostState>,
    event_name_ptr: u32,
    event_name_len: u32,
    event_data_ptr: u32,
    event_data_len: u32,
    flags: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let event_name = read_memory(
        caller.as_context_mut(),
        memory,
        event_name_ptr,
        event_name_len,
    )?;
    let event_data = read_memory(
        caller.as_context_mut(),
        memory,
        event_data_ptr,
        event_data_len,
    )?;
    let event_flags = EventFlags::from_bits(flags).ok_or(InvokeError::SelfError(
        WasmRuntimeError::InvalidEventFlags(flags),
    ))?;

    runtime.actor_emit_event(event_name, event_data, event_flags)
}

fn get_transaction_hash(
    caller: Caller<'_, HostState>,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_, runtime) = grab_runtime!(caller);

    runtime.sys_get_transaction_hash().map(|buffer| buffer.0)
}

fn generate_ruid(caller: Caller<'_, HostState>) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (_, runtime) = grab_runtime!(caller);

    runtime.sys_generate_ruid().map(|buffer| buffer.0)
}

fn emit_log(
    mut caller: Caller<'_, HostState>,
    level_ptr: u32,
    level_len: u32,
    message_ptr: u32,
    message_len: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let level = read_memory(caller.as_context_mut(), memory, level_ptr, level_len)?;
    let message = read_memory(caller.as_context_mut(), memory, message_ptr, message_len)?;

    runtime.sys_log(level, message)
}

fn bech32_encode_address(
    mut caller: Caller<'_, HostState>,
    address_ptr: u32,
    address_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let address = read_memory(caller.as_context_mut(), memory, address_ptr, address_len)?;

    runtime
        .sys_bech32_encode_address(address)
        .map(|buffer| buffer.0)
}

fn panic(
    mut caller: Caller<'_, HostState>,
    message_ptr: u32,
    message_len: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let message = read_memory(caller.as_context_mut(), memory, message_ptr, message_len)?;

    runtime.sys_panic(message)
}

fn bls12381_v1_verify(
    mut caller: Caller<'_, HostState>,
    message_ptr: u32,
    message_len: u32,
    public_key_ptr: u32,
    public_key_len: u32,
    signature_ptr: u32,
    signature_len: u32,
) -> Result<u32, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let message = read_memory(caller.as_context_mut(), memory, message_ptr, message_len)?;
    let public_key = read_memory(
        caller.as_context_mut(),
        memory,
        public_key_ptr,
        public_key_len,
    )?;
    let signature = read_memory(
        caller.as_context_mut(),
        memory,
        signature_ptr,
        signature_len,
    )?;

    runtime.crypto_utils_bls12381_v1_verify(message, public_key, signature)
}

fn bls12381_v1_aggregate_verify(
    mut caller: Caller<'_, HostState>,
    pub_keys_and_msgs_ptr: u32,
    pub_keys_and_msgs_len: u32,
    signature_ptr: u32,
    signature_len: u32,
) -> Result<u32, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let pub_keys_and_msgs = read_memory(
        caller.as_context_mut(),
        memory,
        pub_keys_and_msgs_ptr,
        pub_keys_and_msgs_len,
    )?;
    let signature = read_memory(
        caller.as_context_mut(),
        memory,
        signature_ptr,
        signature_len,
    )?;

    runtime.crypto_utils_bls12381_v1_aggregate_verify(pub_keys_and_msgs, signature)
}

fn bls12381_v1_fast_aggregate_verify(
    mut caller: Caller<'_, HostState>,
    message_ptr: u32,
    message_len: u32,
    public_keys_ptr: u32,
    public_keys_len: u32,
    signature_ptr: u32,
    signature_len: u32,
) -> Result<u32, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let message = read_memory(caller.as_context_mut(), memory, message_ptr, message_len)?;
    let public_keys = read_memory(
        caller.as_context_mut(),
        memory,
        public_keys_ptr,
        public_keys_len,
    )?;
    let signature = read_memory(
        caller.as_context_mut(),
        memory,
        signature_ptr,
        signature_len,
    )?;

    runtime.crypto_utils_bls12381_v1_fast_aggregate_verify(message, public_keys, signature)
}

fn bls12381_g2_signature_aggregate(
    mut caller: Caller<'_, HostState>,
    signatures_ptr: u32,
    signatures_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let signatures = read_memory(
        caller.as_context_mut(),
        memory,
        signatures_ptr,
        signatures_len,
    )?;

    runtime
        .crypto_utils_bls12381_g2_signature_aggregate(signatures)
        .map(|buffer| buffer.0)
}

fn keccak256_hash(
    mut caller: Caller<'_, HostState>,
    data_ptr: u32,
    data_len: u32,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    let (memory, runtime) = grab_runtime!(caller);

    let data = read_memory(caller.as_context_mut(), memory, data_ptr, data_len)?;

    runtime
        .crypto_utils_keccak256_hash(data)
        .map(|buffer| buffer.0)
}

#[cfg(feature = "radix_engine_tests")]
fn test_host_read_memory(
    mut caller: Caller<'_, HostState>,
    memory_offs: u32,
    data_len: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    // - attempt to read data of given length data starting from given memory offset memory_ptr
    let (memory, _runtime) = grab_runtime!(caller);

    read_memory(caller.as_context_mut(), memory, memory_offs, data_len)?;

    Ok(())
}

#[cfg(feature = "radix_engine_tests")]
fn test_host_write_memory(
    mut caller: Caller<'_, HostState>,
    memory_ptr: u32,
    data_len: u32,
) -> Result<(), InvokeError<WasmRuntimeError>> {
    // - generate some random data of of given length data_len
    // - attempt to write this data into given memory offset memory_ptr
    let (memory, _runtime) = grab_runtime!(caller);

    let data = vec![0u8; data_len as usize];
    write_memory(caller.as_context_mut(), memory, memory_ptr, &data)?;

    Ok(())
}

#[cfg(feature = "radix_engine_tests")]
fn test_host_check_memory_is_clean(
    caller: Caller<'_, HostState>,
) -> Result<u64, InvokeError<WasmRuntimeError>> {
    // - attempt to read data of given length data starting from given memory offset memory_ptr
    let (memory, _runtime) = grab_runtime!(caller);
    let store_ctx = caller.as_context();

    let data = memory.data(&store_ctx);
    let clean = !data.iter().any(|&x| x != 0x0);

    Ok(clean as u64)
}
// native functions ends

macro_rules! linker_define {
    ($linker: expr, $name: expr, $var: expr) => {
        $linker
            .define(MODULE_ENV_NAME, $name, $var)
            .expect(stringify!("Failed to define new linker item {}", $name));
    };
}

#[derive(Debug)]
pub enum WasmiInstantiationError {
    ValidationError(Error),
    PreInstantiationError(Error),
    InstantiationError(InstantiationError),
}

impl WasmiModule {
    pub fn new(code: &[u8]) -> Result<Self, WasmiInstantiationError> {
        let engine = Engine::default();
        let mut store = Store::new(&engine, WasmiInstanceEnv::new());

        let module =
            Module::new(&engine, code).map_err(WasmiInstantiationError::ValidationError)?;

        let instance = Self::host_funcs_set(&module, &mut store)
            .map_err(WasmiInstantiationError::PreInstantiationError)?
            .ensure_no_start(store.as_context_mut())
            .map_err(WasmiInstantiationError::InstantiationError)?;

        Ok(Self {
            template_store: unsafe { transmute(store) },
            template_instance: instance,
            code_size_bytes: code.len(),
        })
    }

    pub fn host_funcs_set(
        module: &Module,
        store: &mut Store<HostState>,
    ) -> Result<InstancePre, Error> {
        let host_consume_buffer = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             buffer_id: BufferId,
             destination_ptr: u32|
             -> Result<(), Trap> {
                consume_buffer(caller, buffer_id, destination_ptr).map_err(|e| e.into())
            },
        );

        let host_call_method = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             receiver_ptr: u32,
             receiver_len: u32,
             ident_ptr: u32,
             ident_len: u32,
             args_ptr: u32,
             args_len: u32|
             -> Result<u64, Trap> {
                call_method(
                    caller,
                    receiver_ptr,
                    receiver_len,
                    ident_ptr,
                    ident_len,
                    args_ptr,
                    args_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_call_module_method = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             receiver_ptr: u32,
             receiver_len: u32,
             module_id: u32,
             ident_ptr: u32,
             ident_len: u32,
             args_ptr: u32,
             args_len: u32|
             -> Result<u64, Trap> {
                call_module_method(
                    caller,
                    receiver_ptr,
                    receiver_len,
                    module_id,
                    ident_ptr,
                    ident_len,
                    args_ptr,
                    args_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_call_direct_method = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             receiver_ptr: u32,
             receiver_len: u32,
             ident_ptr: u32,
             ident_len: u32,
             args_ptr: u32,
             args_len: u32|
             -> Result<u64, Trap> {
                call_direct_method(
                    caller,
                    receiver_ptr,
                    receiver_len,
                    ident_ptr,
                    ident_len,
                    args_ptr,
                    args_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_blueprint_call = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             package_address_ptr: u32,
             package_address_len: u32,
             blueprint_name_ptr: u32,
             blueprint_name_len: u32,
             ident_ptr: u32,
             ident_len: u32,
             args_ptr: u32,
             args_len: u32|
             -> Result<u64, Trap> {
                call_function(
                    caller,
                    package_address_ptr,
                    package_address_len,
                    blueprint_name_ptr,
                    blueprint_name_len,
                    ident_ptr,
                    ident_len,
                    args_ptr,
                    args_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_new_component = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             blueprint_name_ptr: u32,
             blueprint_name_len: u32,
             object_states_ptr: u32,
             object_states_len: u32|
             -> Result<u64, Trap> {
                new_object(
                    caller,
                    blueprint_name_ptr,
                    blueprint_name_len,
                    object_states_ptr,
                    object_states_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_new_key_value_store = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             schema_ptr: u32,
             schema_len: u32|
             -> Result<u64, Trap> {
                new_key_value_store(caller, schema_ptr, schema_len).map_err(|e| e.into())
            },
        );

        let host_allocate_global_address = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             package_address_ptr: u32,
             package_address_len: u32,
             blueprint_name_ptr: u32,
             blueprint_name_len: u32|
             -> Result<u64, Trap> {
                allocate_global_address(
                    caller,
                    package_address_ptr,
                    package_address_len,
                    blueprint_name_ptr,
                    blueprint_name_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_get_reservation_address = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             node_id_ptr: u32,
             node_id_len: u32|
             -> Result<u64, Trap> {
                get_reservation_address(caller, node_id_ptr, node_id_len).map_err(|e| e.into())
            },
        );

        let host_execution_cost_unit_limit = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u32, Trap> {
                execution_cost_unit_limit(caller).map_err(|e| e.into())
            },
        );

        let host_execution_cost_unit_price = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u64, Trap> {
                execution_cost_unit_price(caller).map_err(|e| e.into())
            },
        );

        let host_finalization_cost_unit_limit = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u32, Trap> {
                finalization_cost_unit_limit(caller).map_err(|e| e.into())
            },
        );

        let host_finalization_cost_unit_price = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u64, Trap> {
                finalization_cost_unit_price(caller).map_err(|e| e.into())
            },
        );

        let host_usd_price = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u64, Trap> {
                usd_price(caller).map_err(|e| e.into())
            },
        );

        let host_tip_percentage = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u32, Trap> {
                tip_percentage(caller).map_err(|e| e.into())
            },
        );

        let host_fee_balance = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u64, Trap> {
                fee_balance(caller).map_err(|e| e.into())
            },
        );

        let host_globalize_object = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             obj_ptr: u32,
             obj_len: u32,
             modules_ptr: u32,
             modules_len: u32,
             address_ptr: u32,
             address_len: u32|
             -> Result<u64, Trap> {
                globalize_object(
                    caller,
                    obj_ptr,
                    obj_len,
                    modules_ptr,
                    modules_len,
                    address_ptr,
                    address_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_instance_of = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             object_id_ptr: u32,
             object_id_len: u32,
             package_address_ptr: u32,
             package_address_len: u32,
             blueprint_name_ptr: u32,
             blueprint_name_len: u32|
             -> Result<u32, Trap> {
                instance_of(
                    caller,
                    object_id_ptr,
                    object_id_len,
                    package_address_ptr,
                    package_address_len,
                    blueprint_name_ptr,
                    blueprint_name_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_get_blueprint_id = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             object_id_ptr: u32,
             object_id_len: u32|
             -> Result<u64, Trap> {
                blueprint_id(caller, object_id_ptr, object_id_len).map_err(|e| e.into())
            },
        );

        let host_get_outer_object = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             object_id_ptr: u32,
             object_id_len: u32|
             -> Result<u64, Trap> {
                get_outer_object(caller, object_id_ptr, object_id_len).map_err(|e| e.into())
            },
        );

        let host_lock_key_value_store_entry = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             node_id_ptr: u32,
             node_id_len: u32,
             offset_ptr: u32,
             offset_len: u32,
             mutable: u32|
             -> Result<u32, Trap> {
                lock_key_value_store_entry(
                    caller,
                    node_id_ptr,
                    node_id_len,
                    offset_ptr,
                    offset_len,
                    mutable,
                )
                .map_err(|e| e.into())
            },
        );

        let host_key_value_entry_get = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>, handle: u32| -> Result<u64, Trap> {
                key_value_entry_get(caller, handle).map_err(|e| e.into())
            },
        );

        let host_key_value_entry_set = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             handle: u32,
             buffer_ptr: u32,
             buffer_len: u32|
             -> Result<(), Trap> {
                key_value_entry_set(caller, handle, buffer_ptr, buffer_len).map_err(|e| e.into())
            },
        );

        let host_key_value_entry_remove = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>, handle: u32| -> Result<u64, Trap> {
                key_value_entry_remove(caller, handle).map_err(|e| e.into())
            },
        );

        let host_unlock_key_value_entry = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>, handle: u32| -> Result<(), Trap> {
                unlock_key_value_entry(caller, handle).map_err(|e| e.into())
            },
        );

        let host_key_value_store_remove = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             node_id_ptr: u32,
             node_id_len: u32,
             key_ptr: u32,
             key_len: u32|
             -> Result<u64, Trap> {
                key_value_store_remove(caller, node_id_ptr, node_id_len, key_ptr, key_len)
                    .map_err(|e| e.into())
            },
        );

        let host_lock_field = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             object_handle: u32,
             field: u32,
             lock_flags: u32|
             -> Result<u32, Trap> {
                lock_field(caller, object_handle, field, lock_flags).map_err(|e| e.into())
            },
        );

        let host_field_lock_read = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>, handle: u32| -> Result<u64, Trap> {
                field_lock_read(caller, handle).map_err(|e| e.into())
            },
        );

        let host_field_lock_write = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             handle: u32,
             data_ptr: u32,
             data_len: u32|
             -> Result<(), Trap> {
                field_lock_write(caller, handle, data_ptr, data_len).map_err(|e| e.into())
            },
        );

        let host_field_lock_release = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>, handle: u32| -> Result<(), Trap> {
                field_lock_release(caller, handle).map_err(|e| e.into())
            },
        );

        let host_actor_get_node_id = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>, handle: u32| -> Result<u64, Trap> {
                actor_get_node_id(caller, handle).map_err(|e| e.into())
            },
        );

        let host_get_package_address = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u64, Trap> {
                get_package_address(caller).map_err(|e| e.into())
            },
        );

        let host_get_blueprint_name = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u64, Trap> {
                get_blueprint_name(caller).map_err(|e| e.into())
            },
        );

        let host_consume_wasm_execution_units = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>, n: u64| -> Result<(), Trap> {
                consume_wasm_execution_units(caller, n).map_err(|e| e.into())
            },
        );

        let host_emit_event = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             event_name_ptr: u32,
             event_name_len: u32,
             event_data_ptr: u32,
             event_data_len: u32,
             flags: u32|
             -> Result<(), Trap> {
                emit_event(
                    caller,
                    event_name_ptr,
                    event_name_len,
                    event_data_ptr,
                    event_data_len,
                    flags,
                )
                .map_err(|e| e.into())
            },
        );

        let host_emit_log = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             level_ptr: u32,
             level_len: u32,
             message_ptr: u32,
             message_len: u32|
             -> Result<(), Trap> {
                emit_log(caller, level_ptr, level_len, message_ptr, message_len)
                    .map_err(|e| e.into())
            },
        );

        let host_panic = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             message_ptr: u32,
             message_len: u32|
             -> Result<(), Trap> {
                panic(caller, message_ptr, message_len).map_err(|e| e.into())
            },
        );

        let host_bech32_encode_address = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             address_ptr: u32,
             address_len: u32|
             -> Result<u64, Trap> {
                bech32_encode_address(caller, address_ptr, address_len).map_err(|e| e.into())
            },
        );

        let host_get_transaction_hash = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u64, Trap> {
                get_transaction_hash(caller).map_err(|e| e.into())
            },
        );

        let host_generate_ruid = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>| -> Result<u64, Trap> {
                generate_ruid(caller).map_err(|e| e.into())
            },
        );

        let host_bls12381_v1_verify = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             message_ptr: u32,
             message_len: u32,
             public_key_ptr: u32,
             public_key_len: u32,
             signature_ptr: u32,
             signature_len: u32|
             -> Result<u32, Trap> {
                bls12381_v1_verify(
                    caller,
                    message_ptr,
                    message_len,
                    public_key_ptr,
                    public_key_len,
                    signature_ptr,
                    signature_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_bls12381_v1_aggregate_verify = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             pub_keys_and_msgs_ptr: u32,
             pub_keys_and_msgs_len: u32,
             signature_ptr: u32,
             signature_len: u32|
             -> Result<u32, Trap> {
                bls12381_v1_aggregate_verify(
                    caller,
                    pub_keys_and_msgs_ptr,
                    pub_keys_and_msgs_len,
                    signature_ptr,
                    signature_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_bls12381_v1_fast_aggregate_verify = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             message_ptr: u32,
             message_len: u32,
             public_keys_ptr: u32,
             public_keys_len: u32,
             signature_ptr: u32,
             signature_len: u32|
             -> Result<u32, Trap> {
                bls12381_v1_fast_aggregate_verify(
                    caller,
                    message_ptr,
                    message_len,
                    public_keys_ptr,
                    public_keys_len,
                    signature_ptr,
                    signature_len,
                )
                .map_err(|e| e.into())
            },
        );

        let host_bls12381_g2_signature_aggregate = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>,
             signatures_ptr: u32,
             signatures_len: u32|
             -> Result<u64, Trap> {
                bls12381_g2_signature_aggregate(caller, signatures_ptr, signatures_len)
                    .map_err(|e| e.into())
            },
        );

        let host_keccak256_hash = Func::wrap(
            store.as_context_mut(),
            |caller: Caller<'_, HostState>, data_ptr: u32, data_len: u32| -> Result<u64, Trap> {
                keccak256_hash(caller, data_ptr, data_len).map_err(|e| e.into())
            },
        );

        let mut linker = <Linker<HostState>>::new();

        linker_define!(linker, BUFFER_CONSUME_FUNCTION_NAME, host_consume_buffer);
        linker_define!(linker, OBJECT_CALL_FUNCTION_NAME, host_call_method);
        linker_define!(
            linker,
            OBJECT_CALL_MODULE_FUNCTION_NAME,
            host_call_module_method
        );
        linker_define!(
            linker,
            OBJECT_CALL_DIRECT_FUNCTION_NAME,
            host_call_direct_method
        );
        linker_define!(linker, BLUEPRINT_CALL_FUNCTION_NAME, host_blueprint_call);
        linker_define!(linker, OBJECT_NEW_FUNCTION_NAME, host_new_component);

        linker_define!(
            linker,
            ADDRESS_ALLOCATE_FUNCTION_NAME,
            host_allocate_global_address
        );
        linker_define!(
            linker,
            ADDRESS_GET_RESERVATION_ADDRESS_FUNCTION_NAME,
            host_get_reservation_address
        );
        linker_define!(
            linker,
            COSTING_GET_EXECUTION_COST_UNIT_LIMIT_FUNCTION_NAME,
            host_execution_cost_unit_limit
        );
        linker_define!(
            linker,
            COSTING_GET_EXECUTION_COST_UNIT_PRICE_FUNCTION_NAME,
            host_execution_cost_unit_price
        );
        linker_define!(
            linker,
            COSTING_GET_FINALIZATION_COST_UNIT_LIMIT_FUNCTION_NAME,
            host_finalization_cost_unit_limit
        );
        linker_define!(
            linker,
            COSTING_GET_FINALIZATION_COST_UNIT_PRICE_FUNCTION_NAME,
            host_finalization_cost_unit_price
        );
        linker_define!(linker, COSTING_GET_USD_PRICE_FUNCTION_NAME, host_usd_price);
        linker_define!(
            linker,
            COSTING_GET_TIP_PERCENTAGE_FUNCTION_NAME,
            host_tip_percentage
        );
        linker_define!(
            linker,
            COSTING_GET_FEE_BALANCE_FUNCTION_NAME,
            host_fee_balance
        );
        linker_define!(
            linker,
            OBJECT_GLOBALIZE_FUNCTION_NAME,
            host_globalize_object
        );
        linker_define!(linker, OBJECT_INSTANCE_OF_FUNCTION_NAME, host_instance_of);
        linker_define!(
            linker,
            OBJECT_GET_BLUEPRINT_ID_FUNCTION_NAME,
            host_get_blueprint_id
        );
        linker_define!(
            linker,
            OBJECT_GET_OUTER_OBJECT_FUNCTION_NAME,
            host_get_outer_object
        );
        linker_define!(linker, ACTOR_OPEN_FIELD_FUNCTION_NAME, host_lock_field);

        linker_define!(
            linker,
            KEY_VALUE_STORE_NEW_FUNCTION_NAME,
            host_new_key_value_store
        );
        linker_define!(
            linker,
            KEY_VALUE_STORE_OPEN_ENTRY_FUNCTION_NAME,
            host_lock_key_value_store_entry
        );
        linker_define!(
            linker,
            KEY_VALUE_ENTRY_READ_FUNCTION_NAME,
            host_key_value_entry_get
        );
        linker_define!(
            linker,
            KEY_VALUE_ENTRY_WRITE_FUNCTION_NAME,
            host_key_value_entry_set
        );
        linker_define!(
            linker,
            KEY_VALUE_ENTRY_REMOVE_FUNCTION_NAME,
            host_key_value_entry_remove
        );
        linker_define!(
            linker,
            KEY_VALUE_ENTRY_CLOSE_FUNCTION_NAME,
            host_unlock_key_value_entry
        );
        linker_define!(
            linker,
            KEY_VALUE_STORE_REMOVE_ENTRY_FUNCTION_NAME,
            host_key_value_store_remove
        );

        linker_define!(linker, FIELD_ENTRY_READ_FUNCTION_NAME, host_field_lock_read);
        linker_define!(
            linker,
            FIELD_ENTRY_WRITE_FUNCTION_NAME,
            host_field_lock_write
        );
        linker_define!(
            linker,
            FIELD_ENTRY_CLOSE_FUNCTION_NAME,
            host_field_lock_release
        );
        linker_define!(
            linker,
            ACTOR_GET_OBJECT_ID_FUNCTION_NAME,
            host_actor_get_node_id
        );
        linker_define!(
            linker,
            ACTOR_GET_PACKAGE_ADDRESS_FUNCTION_NAME,
            host_get_package_address
        );
        linker_define!(
            linker,
            ACTOR_GET_BLUEPRINT_NAME_FUNCTION_NAME,
            host_get_blueprint_name
        );
        linker_define!(
            linker,
            COSTING_CONSUME_WASM_EXECUTION_UNITS_FUNCTION_NAME,
            host_consume_wasm_execution_units
        );
        linker_define!(linker, ACTOR_EMIT_EVENT_FUNCTION_NAME, host_emit_event);
        linker_define!(linker, SYS_LOG_FUNCTION_NAME, host_emit_log);
        linker_define!(linker, SYS_PANIC_FUNCTION_NAME, host_panic);
        linker_define!(
            linker,
            SYS_GET_TRANSACTION_HASH_FUNCTION_NAME,
            host_get_transaction_hash
        );
        linker_define!(
            linker,
            SYS_BECH32_ENCODE_ADDRESS_FUNCTION_NAME,
            host_bech32_encode_address
        );
        linker_define!(linker, SYS_GENERATE_RUID_FUNCTION_NAME, host_generate_ruid);
        linker_define!(
            linker,
            CRYPTO_UTILS_BLS12381_V1_VERIFY_FUNCTION_NAME,
            host_bls12381_v1_verify
        );
        linker_define!(
            linker,
            CRYPTO_UTILS_BLS12381_V1_AGGREGATE_VERIFY_FUNCTION_NAME,
            host_bls12381_v1_aggregate_verify
        );
        linker_define!(
            linker,
            CRYPTO_UTILS_BLS12381_V1_FAST_AGGREGATE_VERIFY_FUNCTION_NAME,
            host_bls12381_v1_fast_aggregate_verify
        );
        linker_define!(
            linker,
            CRYPTO_UTILS_BLS12381_G2_SIGNATURE_AGGREGATE_FUNCTION_NAME,
            host_bls12381_g2_signature_aggregate
        );

        linker_define!(
            linker,
            CRYPTO_UTILS_KECCAK256_HASH_FUNCTION_NAME,
            host_keccak256_hash
        );

        #[cfg(feature = "radix_engine_tests")]
        {
            let host_read_memory = Func::wrap(
                store.as_context_mut(),
                |caller: Caller<'_, HostState>,
                 memory_offs: u32,
                 data_len: u32|
                 -> Result<(), Trap> {
                    test_host_read_memory(caller, memory_offs, data_len).map_err(|e| e.into())
                },
            );
            let host_write_memory = Func::wrap(
                store.as_context_mut(),
                |caller: Caller<'_, HostState>,
                 memory_offs: u32,
                 data_len: u32|
                 -> Result<(), Trap> {
                    test_host_write_memory(caller, memory_offs, data_len).map_err(|e| e.into())
                },
            );
            let host_check_memory_is_clean = Func::wrap(
                store.as_context_mut(),
                |caller: Caller<'_, HostState>| -> Result<u64, Trap> {
                    test_host_check_memory_is_clean(caller).map_err(|e| e.into())
                },
            );
            linker_define!(linker, "test_host_read_memory", host_read_memory);
            linker_define!(linker, "test_host_write_memory", host_write_memory);
            linker_define!(
                linker,
                "test_host_check_memory_is_clean",
                host_check_memory_is_clean
            );
        }

        linker.instantiate(store.as_context_mut(), &module)
    }

    fn instantiate(&self) -> WasmiInstance {
        let instance = self.template_instance.clone();
        let mut store = self.template_store.clone();
        let memory = match instance.get_export(store.as_context_mut(), EXPORT_MEMORY) {
            Some(Extern::Memory(memory)) => memory,
            _ => panic!("Failed to find memory export"),
        };

        WasmiInstance {
            instance,
            store: unsafe { transmute(store) },
            memory,
        }
    }
}

fn read_memory(
    store: impl AsContextMut,
    memory: Memory,
    ptr: u32,
    len: u32,
) -> Result<Vec<u8>, InvokeError<WasmRuntimeError>> {
    let store_ctx = store.as_context();
    let data = memory.data(&store_ctx);
    let ptr = ptr as usize;
    let len = len as usize;

    if ptr > data.len() || ptr + len > data.len() {
        return Err(InvokeError::SelfError(WasmRuntimeError::MemoryAccessError));
    }
    Ok(data[ptr..ptr + len].to_vec())
}

fn write_memory(
    mut store: impl AsContextMut,
    memory: Memory,
    ptr: u32,
    data: &[u8],
) -> Result<(), InvokeError<WasmRuntimeError>> {
    let mut store_ctx = store.as_context_mut();
    let mem_data = memory.data(&mut store_ctx);

    if ptr as usize > mem_data.len() || ptr as usize + data.len() > mem_data.len() {
        return Err(InvokeError::SelfError(WasmRuntimeError::MemoryAccessError));
    }

    memory
        .write(&mut store.as_context_mut(), ptr as usize, data)
        .or_else(|_| Err(InvokeError::SelfError(WasmRuntimeError::MemoryAccessError)))
}

fn read_slice(
    store: impl AsContextMut,
    memory: Memory,
    v: Slice,
) -> Result<Vec<u8>, InvokeError<WasmRuntimeError>> {
    let ptr = v.ptr();
    let len = v.len();

    read_memory(store, memory, ptr, len)
}

impl WasmiInstance {
    fn get_export_func(&mut self, name: &str) -> Result<Func, InvokeError<WasmRuntimeError>> {
        self.instance
            .get_export(self.store.as_context_mut(), name)
            .and_then(Extern::into_func)
            .ok_or_else(|| {
                InvokeError::SelfError(WasmRuntimeError::UnknownExport(name.to_string()))
            })
    }
}

impl HostError for InvokeError<WasmRuntimeError> {}

impl From<Error> for InvokeError<WasmRuntimeError> {
    fn from(err: Error) -> Self {
        let e_str = format!("{:?}", err);
        match err {
            Error::Trap(trap) => {
                if let Some(invoke_err) = trap.downcast_ref::<InvokeError<WasmRuntimeError>>() {
                    invoke_err.clone()
                } else {
                    InvokeError::SelfError(WasmRuntimeError::ExecutionError(e_str))
                }
            }
            _ => InvokeError::SelfError(WasmRuntimeError::ExecutionError(e_str)),
        }
    }
}

impl WasmInstance for WasmiInstance {
    fn invoke_export<'r>(
        &mut self,
        func_name: &str,
        args: Vec<Buffer>,
        runtime: &mut Box<dyn WasmRuntime + 'r>,
    ) -> Result<Vec<u8>, InvokeError<WasmRuntimeError>> {
        #[cfg(feature = "wasm_fuzzing")]
        if func_name.contains("fuzz") {
            self.fuzz_export(func_name, args, runtime);
            panic!("Fuzzing function executed");
        }
        
        {
            // set up runtime pointer
            // Using triple casting is to workaround this error message:
            // error[E0521]: borrowed data escapes outside of associated function
            //  `runtime` escapes the associated function body here argument requires that `'r` must outlive `'static`
            self.store
                .data_mut()
                .runtime_ptr
                .write(runtime as *mut _ as usize as *mut _);
        }

        let func = self.get_export_func(func_name).unwrap();
        let input: Vec<Value> = args
            .into_iter()
            .map(|buffer| Value::I64(buffer.as_i64()))
            .collect();
        let mut ret = [Value::I64(0)];

        let call_result = func
            .call(self.store.as_context_mut(), &input, &mut ret)
            .map_err(|e| {
                let err: InvokeError<WasmRuntimeError> = e.into();
                err
            });

        let result = match call_result {
            Ok(_) => match i64::try_from(ret[0]) {
                Ok(ret) => read_slice(
                    self.store.as_context_mut(),
                    self.memory,
                    Slice::transmute_i64(ret),
                ),
                _ => Err(InvokeError::SelfError(WasmRuntimeError::InvalidWasmPointer)),
            },
            Err(err) => Err(err),
        };

        #[cfg(feature = "coverage")]
        if let Ok(dump_coverage) = self.get_export_func("dump_coverage") {
            if let Ok(blueprint_buffer) = runtime.actor_get_blueprint_name() {
                let blueprint_name =
                    String::from_utf8(runtime.buffer_consume(blueprint_buffer.id()).unwrap())
                        .unwrap();

                let mut ret = [Value::I64(0)];
                dump_coverage
                    .call(self.store.as_context_mut(), &[], &mut ret)
                    .unwrap();
                let coverage_data = read_slice(
                    self.store.as_context_mut(),
                    self.memory,
                    Slice::transmute_i64(i64::try_from(ret[0]).unwrap()),
                )
                .unwrap();
                save_coverage_data(&blueprint_name, &coverage_data);
            }
        }

        result
    }

    #[cfg(feature = "wasm_fuzzing")]
    fn fuzz_export<'r>(
        &mut self,
        func_name: &str,
        args: Vec<Buffer>,
        runtime: &mut Box<dyn WasmRuntime + 'r>,
    ) {
        use std::{env, process, ptr, io::{self, Read}};
        use nix::{sys::signal::{kill, Signal}, unistd::{close, fork, read, write, ForkResult, Pid, getpid}};
        use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
        use libc;

        // Constants
        const SHM_ENV_VAR: &str = "__AFL_SHM_ID";
        const FORKSRV_FD: i32 = 198;
        const MAP_SIZE: i32 = 1 << 16;

        // init forkserver
        let zeros: [u8; 4] = [0_u8; 4];  // Example data
        write(FORKSRV_FD + 1, &zeros).unwrap();

        let mut child_stopped = false;
        let mut child_pid: Option<Pid> = None;
        
        loop {
            let mut was_killed = [0u8; 4];
            if read(FORKSRV_FD, &mut was_killed).unwrap() != 4 {
                panic!("Forkserver read error");
            }

            if child_stopped && was_killed[0] != 0 {
                child_stopped = false;
                if let Some(pid) = child_pid {
                    if waitpid(pid, None).is_err() {
                        panic!("Failed to wait for child process");
                    }
                }
            }
        
            if !child_stopped {
                match unsafe { fork() } {
                    Ok(ForkResult::Parent { child, .. }) => {
                        child_pid = Some(child);
                    },
                    Ok(ForkResult::Child) => {
                        close(FORKSRV_FD).unwrap();
                        close(FORKSRV_FD + 1).unwrap();
                        break;
                    },
                    Err(_) => panic!("Fork failed"),
                }
            } else {
                // Restart child with SIGCONT
                println!("Restarting child");
                if let Some(pid) = child_pid {
                    kill(pid, Signal::SIGCONT).unwrap();
                    child_stopped = false;
                }
            }

            if let Some(pid) = child_pid {
                if write(FORKSRV_FD + 1, &pid.as_raw().to_ne_bytes()).unwrap() != 4 {
                    panic!("Forkserver write error");
                }
    
                let mut status: i32 = 0;

                match waitpid(pid, None).unwrap() {
                    WaitStatus::Stopped(_, _) => {
                        child_stopped = true;
                    }
                    WaitStatus::Exited(_, code) => {
                        status = code;
                    }
                    _ => {}
                }
    
                // Relay wait status to pipe

                //if libc::WIFSTOPPED(pid.as_raw()) {
                //    child_stopped = true;
                //}
    
                if write(FORKSRV_FD + 1, &status.to_ne_bytes()).unwrap() != 4 {
                    panic!("Forkserver write error");
                }
            }
        }

        // fuzzing
        {
            self.store
                .data_mut()
                .runtime_ptr
                .write(runtime as *mut _ as usize as *mut _);
        }

        let func = self.get_export_func(func_name).unwrap();

        let mut raw_input : Vec<u8> = Vec::new();
        let result = io::stdin().read_to_end(&mut raw_input);
        if result.is_err() {
            panic!("Failed to read input");
            return;
        }

        //println!("Input: {:?}", raw_input);

        if raw_input.len() == 4 && raw_input[0] == 'f' as u8 {
            println!("Sum should be higher");
        }

        let input = scrypto_encode(&(raw_input, )).unwrap();
        let buffer_id = runtime.allocate_buffer(input).unwrap();
        let input = vec![Value::I64(buffer_id.as_i64())];
        let mut ret = [Value::I64(0)];

        let call_result = func
            .call(self.store.as_context_mut(), &input, &mut ret)
            .map_err(|e| {
                let err: InvokeError<WasmRuntimeError> = e.into();
                err
            });

        let result = match call_result {
            Ok(_) => match i64::try_from(ret[0]) {
                Ok(ret) => read_slice(
                    self.store.as_context_mut(),
                    self.memory,
                    Slice::transmute_i64(ret),
                ),
                _ => Err(InvokeError::SelfError(WasmRuntimeError::InvalidWasmPointer)),
            },
            Err(err) => Err(err),
        };

        if result.is_err() {
            panic!("Failed to execute fuzzing function");
        };

        let dump_coverage_func = self.get_export_func("dump_coverage_counters").unwrap();
        let mut ret = [Value::I64(0)];
        dump_coverage_func
            .call(self.store.as_context_mut(), &[], &mut ret)
            .expect("Failed to call dump_coverage");
        let coverage_data = read_slice(
            self.store.as_context_mut(),
            self.memory,
            Slice::transmute_i64(i64::try_from(ret[0]).unwrap()),
        )
        .expect("Failed to read coverage data");

        let id_str = env::var(SHM_ENV_VAR).unwrap();
        let shm_id = id_str.parse::<i32>().unwrap();
    
        let afl_area = unsafe {
            let addr = libc::shmat(shm_id, ptr::null_mut(), 0);
            if addr.is_null() {
                panic!("Failed to attach to shared memory: {}", io::Error::last_os_error());
            }            
            let area_ptr = addr as *mut u8;
            std::slice::from_raw_parts_mut(area_ptr, MAP_SIZE as usize)
        };

        for i in 0..coverage_data.len() {
            afl_area[i] = afl_area[i].wrapping_add(coverage_data[i]);
        }
        let sum = afl_area.iter().fold(0, |acc, &x| acc + x as i32);
        if sum > 50 {
            println!("Coverage sum: {}", sum);
        }

        std::process::exit(0);
    }    
}

#[derive(Debug, Clone)]
pub struct WasmiEngineOptions {
    max_cache_size: usize,
}

pub struct WasmiEngine {
    // This flag disables cache in wasm_instrumenter/wasmi/wasmer to prevent non-determinism when fuzzing
    #[cfg(all(not(feature = "fuzzing"), not(feature = "moka")))]
    modules_cache: RefCell<lru::LruCache<CodeHash, Arc<WasmiModule>>>,
    #[cfg(all(not(feature = "fuzzing"), feature = "moka"))]
    modules_cache: moka::sync::Cache<CodeHash, Arc<WasmiModule>>,
    #[cfg(feature = "fuzzing")]
    #[allow(dead_code)]
    modules_cache: usize,
}

impl Default for WasmiEngine {
    fn default() -> Self {
        Self::new(WasmiEngineOptions {
            max_cache_size: WASM_ENGINE_CACHE_SIZE,
        })
    }
}

impl WasmiEngine {
    pub fn new(options: WasmiEngineOptions) -> Self {
        #[cfg(all(not(feature = "fuzzing"), not(feature = "moka")))]
        let modules_cache = RefCell::new(lru::LruCache::new(
            sbor::rust::num::NonZeroUsize::new(options.max_cache_size).unwrap(),
        ));
        #[cfg(all(not(feature = "fuzzing"), feature = "moka"))]
        let modules_cache = moka::sync::Cache::builder()
            .weigher(|_key: &CodeHash, _value: &Arc<WasmiModule>| -> u32 {
                // No sophisticated weighing mechanism, just keep a fixed size cache
                1u32
            })
            .max_capacity(options.max_cache_size as u64)
            .build();
        #[cfg(feature = "fuzzing")]
        let modules_cache = options.max_cache_size;

        Self { modules_cache }
    }
}

impl WasmEngine for WasmiEngine {
    type WasmInstance = WasmiInstance;

    #[allow(unused_variables)]
    fn instantiate(&self, code_hash: CodeHash, instrumented_code: &[u8]) -> WasmiInstance {
        #[cfg(not(feature = "fuzzing"))]
        {
            #[cfg(not(feature = "moka"))]
            {
                if let Some(cached_module) = self.modules_cache.borrow_mut().get(&code_hash) {
                    return cached_module.instantiate();
                }
            }
            #[cfg(feature = "moka")]
            if let Some(cached_module) = self.modules_cache.get(&code_hash) {
                return cached_module.as_ref().instantiate();
            }
        }

        let module = WasmiModule::new(instrumented_code).expect("Failed to instantiate module");
        let instance = module.instantiate();

        #[cfg(not(feature = "fuzzing"))]
        {
            #[cfg(not(feature = "moka"))]
            self.modules_cache
                .borrow_mut()
                .put(code_hash, Arc::new(module));
            #[cfg(feature = "moka")]
            self.modules_cache.insert(code_hash, Arc::new(module));
        }

        instance
    }
}

// Below tests verify WASM "mutable-global" feature, which allows importing/exporting mutable globals.
// more details:
// - https://github.com/WebAssembly/mutable-global/blob/master/proposals/mutable-global/Overview.md

// NOTE!
//  We test only WASM code, because Rust currently does not use the WASM "global" construct for globals
//  (it places them into the linear memory instead).
//  more details:
//  - https://github.com/rust-lang/rust/issues/60825
//  - https://github.com/rust-lang/rust/issues/65987
#[cfg(not(feature = "wasmer"))]
#[cfg(test)]
mod tests {
    use super::*;
    use wabt::{wat2wasm, wat2wasm_with_features, ErrorKind, Features};
    use wasmi::Global;

    static MODULE_MUTABLE_GLOBALS: &str = r#"
            (module
                ;; below line is invalid if feature 'Import/Export mutable globals' is disabled
                ;; see: https://github.com/WebAssembly/mutable-global/blob/master/proposals/mutable-global/Overview.md
                (global $g (import "env" "global_mutable_value") (mut i32))

                ;; Simple function that always returns `0`
                (func $increase_global_value (param $step i32) (result i32)

                    (global.set $g
                        (i32.add
                            (global.get $g)
                            (local.get $step)))

                    (i32.const 0)
                )
                (memory $0 1)
                (export "memory" (memory $0))
                (export "increase_global_value" (func $increase_global_value))
            )
        "#;

    // This test is not wasmi-specific, but decided to put it here along with next one
    #[test]
    fn test_wasm_non_mvp_mutable_globals_build_with_feature_disabled() {
        let mut features = Features::new();
        features.disable_mutable_globals();

        assert!(
            match wat2wasm_with_features(MODULE_MUTABLE_GLOBALS, features) {
                Err(err) => {
                    match err.kind() {
                        ErrorKind::Validate(msg) => {
                            msg.contains("mutable globals cannot be imported")
                        }
                        _ => false,
                    }
                }
                Ok(_) => false,
            }
        )
    }
    pub fn run_module_with_mutable_global(
        engine: &Engine,
        mut store: StoreContextMut<WasmiInstanceEnv>,
        code: &[u8],
        func_name: &str,
        global_name: &str,
        global_value: &Global,
        step: i32,
    ) {
        let module = Module::new(&engine, code).unwrap();

        let mut linker = <Linker<HostState>>::new();
        linker_define!(linker, global_name, *global_value);

        let instance = linker
            .instantiate(store.as_context_mut(), &module)
            .unwrap()
            .ensure_no_start(store.as_context_mut())
            .unwrap();

        let func = instance
            .get_export(store.as_context_mut(), func_name)
            .and_then(Extern::into_func)
            .unwrap();

        let input = [Value::I32(step)];
        let mut ret = [Value::I32(0)];

        let _ = func.call(store.as_context_mut(), &input, &mut ret);
    }

    #[test]
    fn test_wasm_non_mvp_mutable_globals_execute_code() {
        // wat2wasm has "mutable-globals" enabled by default
        let code = wat2wasm(MODULE_MUTABLE_GLOBALS).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, WasmiInstanceEnv::new());

        // Value of this Global shall be updated by the below WASM module calls
        let global_value = Global::new(store.as_context_mut(), Value::I32(100), Mutability::Var);

        run_module_with_mutable_global(
            &engine,
            store.as_context_mut(),
            &code,
            "increase_global_value",
            "global_mutable_value",
            &global_value,
            1000,
        );
        let updated_value = global_value.get(store.as_context());
        let val = i32::try_from(updated_value).unwrap();
        assert_eq!(val, 1100);

        run_module_with_mutable_global(
            &engine,
            store.as_context_mut(),
            &code,
            "increase_global_value",
            "global_mutable_value",
            &global_value,
            10000,
        );
        let updated_value = global_value.get(store.as_context());
        let val = i32::try_from(updated_value).unwrap();
        assert_eq!(val, 11100);
    }
}
