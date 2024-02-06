use radix_engine::errors::RuntimeError;
use radix_engine::kernel::call_frame::CallFrameMessage;
use radix_engine::kernel::id_allocator::IdAllocator;
use radix_engine::kernel::kernel::{BootLoader, Kernel};
use radix_engine::kernel::kernel_api::{
    KernelApi, KernelInternalApi, KernelInvocation, KernelInvokeApi, KernelNodeApi,
    KernelSubstateApi,
};
use radix_engine::kernel::kernel_callback_api::{
    CallFrameReferences, CloseSubstateEvent, CreateNodeEvent, DrainSubstatesEvent, DropNodeEvent,
    KernelCallbackObject, MoveModuleEvent, OpenSubstateEvent, ReadSubstateEvent,
    RemoveSubstateEvent, ScanKeysEvent, ScanSortedSubstatesEvent, SetSubstateEvent,
    WriteSubstateEvent,
};
use radix_engine::system::checkers::KernelDatabaseChecker;
use radix_engine::track::{to_state_updates, BootStore, CommitableSubstateStore, Track};
use radix_engine::types::*;
use radix_engine_store_interface::db_key_mapper::SpreadPrefixKeyMapper;
use radix_engine_store_interface::interface::CommittableSubstateDatabase;
use radix_engine_stores::memory_db::InMemorySubstateDatabase;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use transaction::model::PreAllocatedAddress;

struct TestCallFrameData;

impl CallFrameReferences for TestCallFrameData {
    fn root() -> Self {
        TestCallFrameData
    }

    fn global_references(&self) -> Vec<NodeId> {
        Default::default()
    }

    fn direct_access_references(&self) -> Vec<NodeId> {
        Default::default()
    }

    fn stable_transient_references(&self) -> Vec<NodeId> {
        Default::default()
    }

    fn len(&self) -> usize {
        0usize
    }
}

struct TestCallbackObject;
impl KernelCallbackObject for TestCallbackObject {
    type LockData = ();
    type CallFrameData = TestCallFrameData;
    type CallbackState = ();

    fn start<Y>(
        _: &mut Y,
        _: &[u8],
        _: &Vec<PreAllocatedAddress>,
        _: &IndexSet<Reference>,
        _: &IndexMap<Hash, Vec<u8>>,
    ) -> Result<Vec<u8>, RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        unreachable!()
    }

    fn init<S: BootStore>(&mut self, _store: &S) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn on_teardown<Y>(_api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(())
    }

    fn on_pin_node(&mut self, _node_id: &NodeId) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn on_create_node<Y>(_api: &mut Y, _event: CreateNodeEvent) -> Result<(), RuntimeError>
    where
        Y: KernelInternalApi<Self>,
    {
        Ok(())
    }

    fn on_drop_node<Y>(_api: &mut Y, _event: DropNodeEvent) -> Result<(), RuntimeError>
    where
        Y: KernelInternalApi<Self>,
    {
        Ok(())
    }

    fn on_move_module<Y>(_api: &mut Y, _event: MoveModuleEvent) -> Result<(), RuntimeError>
    where
        Y: KernelInternalApi<Self>,
    {
        Ok(())
    }

    fn on_open_substate<Y>(_api: &mut Y, _event: OpenSubstateEvent) -> Result<(), RuntimeError>
    where
        Y: KernelInternalApi<Self>,
    {
        Ok(())
    }

    fn on_close_substate<Y>(_api: &mut Y, _event: CloseSubstateEvent) -> Result<(), RuntimeError>
    where
        Y: KernelInternalApi<Self>,
    {
        Ok(())
    }

    fn on_read_substate<Y>(_api: &mut Y, _event: ReadSubstateEvent) -> Result<(), RuntimeError>
    where
        Y: KernelInternalApi<Self>,
    {
        Ok(())
    }

    fn on_write_substate<Y>(_api: &mut Y, _event: WriteSubstateEvent) -> Result<(), RuntimeError>
    where
        Y: KernelInternalApi<Self>,
    {
        Ok(())
    }

    fn on_set_substate(&mut self, _event: SetSubstateEvent) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn on_remove_substate(&mut self, _event: RemoveSubstateEvent) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn on_scan_keys(&mut self, _event: ScanKeysEvent) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn on_drain_substates(&mut self, _event: DrainSubstatesEvent) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn on_scan_sorted_substates(
        &mut self,
        _event: ScanSortedSubstatesEvent,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn before_invoke<Y>(
        _invocation: &KernelInvocation<Self::CallFrameData>,
        _api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(())
    }

    fn after_invoke<Y>(_output: &IndexedScryptoValue, _api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(())
    }

    fn on_execution_start<Y>(_api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(())
    }

    fn on_execution_finish<Y>(_message: &CallFrameMessage, _api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(())
    }

    fn on_allocate_node_id<Y>(_entity_type: EntityType, _api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(())
    }

    fn invoke_upstream<Y>(
        args: &IndexedScryptoValue,
        _api: &mut Y,
    ) -> Result<IndexedScryptoValue, RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(args.clone())
    }

    fn auto_drop<Y>(_nodes: Vec<NodeId>, _api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(())
    }

    fn on_mark_substate_as_transient(
        &mut self,
        _node_id: &NodeId,
        _partition_number: &PartitionNumber,
        _substate_key: &SubstateKey,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn on_substate_lock_fault<Y>(
        _node_id: NodeId,
        _partition_num: PartitionNumber,
        _offset: &SubstateKey,
        _api: &mut Y,
    ) -> Result<bool, RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(false)
    }

    fn on_drop_node_mut<Y>(_node_id: &NodeId, _api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(())
    }

    fn on_move_node<Y>(
        _node_id: &NodeId,
        _is_moving_down: bool,
        _is_to_barrier: bool,
        _destination_blueprint_id: Option<BlueprintId>,
        _api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: KernelApi<Self>,
    {
        Ok(())
    }
}

struct KernelFuzzer {
    rng: ChaCha8Rng,
    allocated_nodes: Vec<NodeId>,
    nodes: Vec<NodeId>,
    handles: Vec<SubstateHandle>,
}

impl KernelFuzzer {
    fn new(seed: u64) -> Self {
        KernelFuzzer {
            rng: ChaCha8Rng::seed_from_u64(seed),
            allocated_nodes: Vec::new(),
            nodes: Vec::new(),
            handles: Vec::new(),
        }
    }

    fn add_allocated_node(&mut self, node_id: NodeId) {
        self.allocated_nodes.push(node_id);
    }

    fn next_allocated_node(&mut self) -> Option<NodeId> {
        if self.allocated_nodes.is_empty() {
            None
        } else {
            let index = self.rng.gen_range(0usize..self.allocated_nodes.len());
            let node_id = self.allocated_nodes.remove(index);
            self.nodes.push(node_id);
            Some(node_id)
        }
    }

    fn next_node(&mut self) -> Option<NodeId> {
        if self.nodes.is_empty() {
            None
        } else {
            let index = self.rng.gen_range(0usize..self.nodes.len());
            Some(self.nodes[index])
        }
    }

    fn add_handle(&mut self, handle: SubstateHandle) {
        self.handles.push(handle);
    }

    fn next_handle(&mut self) -> Option<SubstateHandle> {
        if self.handles.is_empty() {
            None
        } else {
            let index = self.rng.gen_range(0usize..self.handles.len());
            Some(self.handles[index])
        }
    }

    fn remove_next_handle(&mut self) -> Option<SubstateHandle> {
        if self.handles.is_empty() {
            None
        } else {
            let index = self.rng.gen_range(0usize..self.handles.len());
            Some(self.handles.remove(index))
        }
    }

    fn next_value(&mut self) -> IndexedScryptoValue {
        let mut owned = Vec::new();
        let mut refs = Vec::new();
        let num_children = self.rng.gen_range(0usize..self.nodes.len());
        for _ in 0usize..num_children {
            let index = self.rng.gen_range(0usize..self.nodes.len());
            if self.rng.gen_bool(0.5) {
                owned.push(Own(self.nodes[index]));
            } else {
                refs.push(Reference(self.nodes[index]));
            }
        }

        IndexedScryptoValue::from_typed(&(owned, refs))
    }

    fn next_entity_type(&mut self) -> EntityType {
        if self.rng.gen_bool(0.5) {
            EntityType::InternalKeyValueStore
        } else {
            EntityType::GlobalAccount
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, FromRepr, Ord, PartialOrd, Eq, PartialEq)]
enum KernelFuzzAction {
    Allocate,
    CreateNode,
    PinNode,
    DropNode,
    Invoke,
    CreateNodeFrom,
    MarkSubstateAsTransient,
    OpenSubstate,
    ReadSubstate,
    WriteSubstate,
    CloseSubstate,
}

impl KernelFuzzAction {
    fn execute<S>(
        &self,
        fuzzer: &mut KernelFuzzer,
        kernel: &mut Kernel<'_, TestCallbackObject, S>,
    ) -> Result<bool, RuntimeError>
    where
        S: CommitableSubstateStore,
    {
        return match self {
            KernelFuzzAction::Allocate => {
                if (fuzzer.nodes.len() + fuzzer.allocated_nodes.len()) < 4 {
                    let node_id = kernel.kernel_allocate_node_id(fuzzer.next_entity_type())?;
                    fuzzer.add_allocated_node(node_id);
                    return Ok(false);
                }

                Ok(true)
            }
            KernelFuzzAction::CreateNode => {
                if let Some(node_id) = fuzzer.next_allocated_node() {
                    let value = fuzzer.next_value();
                    let value2 = fuzzer.next_value();
                    let substates = btreemap!(
                        PartitionNumber(0u8) => btreemap!(
                            SubstateKey::Field(0u8) => value
                        ),
                        PartitionNumber(1u8) => btreemap!(
                            SubstateKey::Field(0u8) => value2
                        ),
                    );
                    kernel.kernel_create_node(node_id, substates)?;
                    return Ok(false);
                }

                Ok(true)
            }
            KernelFuzzAction::CreateNodeFrom => {
                if let Some(src) = fuzzer.next_node().filter(|n| !n.is_global()) {
                    if let Some(dest) = fuzzer.next_allocated_node() {
                        kernel.kernel_create_node_from(
                            dest,
                            btreemap!(PartitionNumber(1u8) => (src, PartitionNumber(1u8))),
                        )?;
                        let value = fuzzer.next_value();
                        let handle = kernel.kernel_open_substate_with_default(
                            &dest,
                            PartitionNumber(0u8),
                            &SubstateKey::Field(0u8),
                            LockFlags::MUTABLE,
                            Some(|| IndexedScryptoValue::from_typed(&())),
                            (),
                        )?;
                        kernel.kernel_write_substate(handle, value)?;
                        kernel.kernel_close_substate(handle)?;

                        return Ok(false);
                    }
                }

                Ok(true)
            }
            KernelFuzzAction::PinNode => {
                if let Some(node_id) = fuzzer.next_node() {
                    kernel.kernel_pin_node(node_id)?;
                    return Ok(false);
                }

                Ok(true)
            }
            KernelFuzzAction::DropNode => {
                if let Some(node_id) = fuzzer.next_node() {
                    kernel.kernel_drop_node(&node_id)?;
                    return Ok(false);
                }

                Ok(true)
            }
            KernelFuzzAction::Invoke => {
                if let Some(node_id) = fuzzer.next_node() {
                    let invocation = KernelInvocation {
                        call_frame_data: TestCallFrameData,
                        args: IndexedScryptoValue::from_typed(&Own(node_id)),
                    };
                    kernel.kernel_invoke(Box::new(invocation))?;
                    return Ok(false);
                }

                Ok(true)
            }
            KernelFuzzAction::MarkSubstateAsTransient => {
                if let Some(node_id) = fuzzer.next_node() {
                    kernel.kernel_mark_substate_as_transient(
                        node_id,
                        PartitionNumber(1u8),
                        SubstateKey::Field(1u8),
                    )?;
                    return Ok(false);
                }

                Ok(true)
            }
            KernelFuzzAction::OpenSubstate => {
                if let Some(node_id) = fuzzer.next_node() {
                    let partition = fuzzer.rng.gen_range(0u8..1u8);
                    let handle = kernel.kernel_open_substate(
                        &node_id,
                        PartitionNumber(partition),
                        &SubstateKey::Field(0u8),
                        LockFlags::read_only(),
                        (),
                    )?;
                    fuzzer.add_handle(handle);
                    return Ok(false);
                }

                Ok(true)
            }
            KernelFuzzAction::ReadSubstate => {
                if let Some(handle) = fuzzer.next_handle() {
                    kernel.kernel_read_substate(handle)?;
                    return Ok(false);
                }

                Ok(true)
            }
            KernelFuzzAction::WriteSubstate => {
                if let Some(handle) = fuzzer.next_handle() {
                    let value = fuzzer.next_value();
                    kernel.kernel_write_substate(handle, value)?;
                    return Ok(false);
                }

                Ok(true)
            }
            KernelFuzzAction::CloseSubstate => {
                if let Some(handle) = fuzzer.remove_next_handle() {
                    kernel.kernel_close_substate(handle)?;
                    return Ok(false);
                }

                Ok(true)
            }
        };
    }
}

fn kernel_fuzz<F: FnMut(&mut KernelFuzzer) -> Vec<KernelFuzzAction>>(
    seed: u64,
    mut action_generator: F,
) -> Result<(), RuntimeError> {
    let txn_hash = &seed.to_be_bytes().repeat(4)[..];
    let mut id_allocator = IdAllocator::new(Hash(txn_hash.try_into().unwrap()), [0u32; 256]);
    let mut substate_db = InMemorySubstateDatabase::standard();
    let mut track = Track::<InMemorySubstateDatabase, SpreadPrefixKeyMapper>::new(&substate_db);
    let mut callback = TestCallbackObject;
    let mut boot_loader = BootLoader {
        id_allocator: &mut id_allocator,
        callback: &mut callback,
        store: &mut track,
    };
    let mut kernel = boot_loader.boot()?;

    let mut fuzzer = KernelFuzzer::new(seed);

    let actions = action_generator(&mut fuzzer);

    for action in &actions {
        match action.execute(&mut fuzzer, &mut kernel) {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }
    }

    let result = track.finalize();
    if let Ok((tracked_nodes, deleted_partitions)) = result {
        let state_updates =
            to_state_updates::<SpreadPrefixKeyMapper>(tracked_nodes, deleted_partitions);

        let database_updates = state_updates.create_database_updates::<SpreadPrefixKeyMapper>();
        substate_db.commit(&database_updates);
        let mut checker = KernelDatabaseChecker::new();
        checker.check_db(&substate_db).unwrap_or_else(|_| {
            panic!(
                "Database is not consistent at seed: {:?} actions: {:?}",
                seed, actions
            )
        });
    }

    Ok(())
}

#[test]
fn fuzz_from_one_node() {
    let success_count = (0u64..1_000_000u64)
        .into_par_iter()
        .map(|seed| {
            let result = kernel_fuzz(seed, |fuzzer| {
                let mut actions = vec![KernelFuzzAction::Allocate, KernelFuzzAction::CreateNode];
                for _ in 0..8 {
                    let action =
                        KernelFuzzAction::from_repr(fuzzer.rng.gen_range(0u8..=10u8)).unwrap();
                    actions.push(action);
                }
                actions
            });

            if result.is_ok() {
                1
            } else {
                0
            }
        })
        .reduce(|| 0, |acc, e| acc + e);

    println!("Success Count: {:?}", success_count);
}

#[test]
fn fuzz_from_two_open_substates() {
    let success_count = (0u64..1_000_000u64)
        .into_par_iter()
        .map(|seed| {
            let result = kernel_fuzz(seed, |fuzzer| {
                let mut actions = vec![
                    KernelFuzzAction::Allocate,
                    KernelFuzzAction::CreateNode,
                    KernelFuzzAction::Allocate,
                    KernelFuzzAction::CreateNode,
                    KernelFuzzAction::OpenSubstate,
                    KernelFuzzAction::OpenSubstate,
                ];
                for _ in 0..4 {
                    let action =
                        KernelFuzzAction::from_repr(fuzzer.rng.gen_range(0u8..=10u8)).unwrap();
                    actions.push(action);
                }
                actions
            });

            if result.is_ok() {
                1
            } else {
                0
            }
        })
        .reduce(|| 0, |acc, e| acc + e);

    println!("Success Count: {:?}", success_count);
}

/// Reproduced the close substate bug
#[test]
fn fuzz_node_three_chain() {
    let success_count = (0u64..100_000u64)
        .into_par_iter()
        .map(|seed| {
            let result = kernel_fuzz(seed, |_fuzzer| {
                vec![
                    KernelFuzzAction::Allocate,
                    KernelFuzzAction::CreateNode,
                    KernelFuzzAction::Allocate,
                    KernelFuzzAction::CreateNode,
                    KernelFuzzAction::OpenSubstate,
                    KernelFuzzAction::OpenSubstate,
                    KernelFuzzAction::CloseSubstate,
                    KernelFuzzAction::Allocate,
                    KernelFuzzAction::CreateNode,
                    KernelFuzzAction::ReadSubstate,
                ]
            });

            if result.is_ok() {
                1
            } else {
                0
            }
        })
        .reduce(|| 0, |acc, e| acc + e);

    println!("Success Count: {:?}", success_count);
}
