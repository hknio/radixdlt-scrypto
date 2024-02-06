use radix_engine::errors::{CallFrameError, KernelError, RuntimeError};
use radix_engine::kernel::call_frame::OpenSubstateError;
use radix_engine::kernel::id_allocator::IdAllocator;
use radix_engine::kernel::kernel::BootLoader;
use radix_engine::kernel::kernel_api::KernelSubstateApi;
use radix_engine::system::bootstrap::Bootstrapper;
use radix_engine::system::system_callback::{SystemConfig, SystemLockData};
use radix_engine::system::system_modules::costing::{FeeTable, SystemLoanFeeReserve};
use radix_engine::system::system_modules::SystemModuleMixer;
use radix_engine::track::Track;
use radix_engine::transaction::ExecutionConfig;
use radix_engine::types::*;
use radix_engine::vm::wasm::DefaultWasmEngine;
use radix_engine::vm::{DefaultNativeVm, ScryptoVm, Vm};
use radix_engine_interface::api::LockFlags;
use radix_engine_queries::typed_substate_layout::{
    BlueprintVersionKey, PACKAGE_AUTH_TEMPLATE_PARTITION_OFFSET,
};
use radix_engine_store_interface::db_key_mapper::SpreadPrefixKeyMapper;
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use transaction::prelude::*;

#[test]
pub fn test_open_substate_of_invisible_package_address() {
    // Create dummy transaction
    let transaction =
        TestTransaction::new_from_nonce(ManifestBuilder::new().lock_fee_from_faucet().build(), 1)
            .prepare()
            .unwrap();
    let executable = transaction.get_executable(btreeset![]);
    let execution_config = ExecutionConfig::for_test_transaction();

    // Create database and bootstrap
    let mut database = InMemorySubstateDatabase::standard();
    let scrypto_vm = ScryptoVm::<DefaultWasmEngine>::default();
    let native_vm = DefaultNativeVm::new();
    let vm = Vm {
        scrypto_vm: &scrypto_vm,
        native_vm: native_vm.clone(),
    };
    Bootstrapper::new(NetworkDefinition::simulator(), &mut database, vm, false);

    // Create kernel
    let mut id_allocator = IdAllocator::new(executable.intent_hash().to_hash(), DEFAULT_NEXT_NODE_IDS);
    let mut system = SystemConfig {
        blueprint_cache: NonIterMap::new(),
        auth_cache: NonIterMap::new(),
        schema_cache: NonIterMap::new(),
        callback_obj: Vm {
            scrypto_vm: &scrypto_vm,
            native_vm: native_vm,
        },
        modules: SystemModuleMixer::new(
            execution_config.enabled_modules,
            NetworkDefinition::simulator(),
            executable.intent_hash().to_hash(),
            executable.auth_zone_params().clone(),
            SystemLoanFeeReserve::default(),
            FeeTable::new(),
            executable.payload_size(),
            executable.auth_zone_params().initial_proofs.len(),
            &execution_config,
        ),
    };
    let mut track = Track::<InMemorySubstateDatabase, SpreadPrefixKeyMapper>::new(&database);
    let mut boot_loader = BootLoader {
        id_allocator: &mut id_allocator,
        callback: &mut system,
        store: &mut track,
    };
    let mut kernel = boot_loader.boot().unwrap();

    // Lock package substate
    let result = kernel.kernel_open_substate(
        PACKAGE_PACKAGE.as_node_id(),
        MAIN_BASE_PARTITION
            .at_offset(PACKAGE_AUTH_TEMPLATE_PARTITION_OFFSET)
            .unwrap(),
        &SubstateKey::Map(scrypto_encode(&BlueprintVersionKey::new_default("Test")).unwrap()),
        LockFlags::read_only(),
        SystemLockData::default(),
    );

    // Verify lock substate
    assert!(matches!(
        result,
        Err(RuntimeError::KernelError(KernelError::CallFrameError(
            CallFrameError::OpenSubstateError(OpenSubstateError::SubstateFault)
        )))
    ));
}
