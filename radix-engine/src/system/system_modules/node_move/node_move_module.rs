use crate::blueprints::resource::ProofMoveableSubstate;
use crate::errors::{ModuleError, RuntimeError};
use crate::kernel::actor::Actor;
use crate::kernel::call_frame::CallFrameUpdate;
use crate::kernel::kernel_api::KernelApi;
use crate::kernel::kernel_callback_api::KernelCallbackObject;
use crate::system::module::SystemModule;
use crate::system::node_modules::type_info::{TypeInfoBlueprint, TypeInfoSubstate};
use crate::system::system_callback::{SystemConfig, SystemLockData};
use crate::system::system_callback_api::SystemCallbackObject;
use crate::types::*;
use radix_engine_interface::api::LockFlags;
use radix_engine_interface::blueprints::resource::*;
use radix_engine_interface::*;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoSbor)]
pub enum NodeMoveError {
    CantMoveDownstream(NodeId),
    CantMoveUpstream(NodeId),
}

#[derive(Debug, Clone)]
pub struct NodeMoveModule {}

impl NodeMoveModule {
    fn prepare_move_downstream<Y: KernelApi<SystemConfig<V>>, V: SystemCallbackObject>(
        node_id: NodeId,
        callee: &Actor,
        api: &mut Y,
    ) -> Result<(), RuntimeError> {
        // TODO: Make this more generic?
        let type_info = TypeInfoBlueprint::get_type(&node_id, api)?;
        match type_info {
            TypeInfoSubstate::Object(ObjectInfo {
                blueprint,
                outer_object,
                ..
            }) if blueprint.package_address.eq(&RESOURCE_MANAGER_PACKAGE)
                && blueprint.blueprint_name.eq(FUNGIBLE_PROOF_BLUEPRINT) =>
            {
                if matches!(callee, Actor::Method { node_id, .. } if node_id.eq(outer_object.unwrap().as_node_id()))
                {
                    return Ok(());
                }

                if matches!(callee, Actor::Function { .. }) && callee.blueprint().eq(&blueprint) {
                    return Ok(());
                }

                // Change to restricted unless it's moved to auth zone.
                // TODO: align with barrier design?
                let mut changed_to_restricted = true;
                if let Actor::Method { node_id, .. } = callee {
                    let type_info = TypeInfoBlueprint::get_type(node_id, api)?;
                    if let TypeInfoSubstate::Object(ObjectInfo { blueprint, .. }) = type_info {
                        if blueprint.eq(&Blueprint::new(
                            &RESOURCE_MANAGER_PACKAGE,
                            AUTH_ZONE_BLUEPRINT,
                        )) {
                            changed_to_restricted = false;
                        }
                    }
                }

                let handle = api.kernel_lock_substate(
                    &node_id,
                    OBJECT_BASE_MODULE,
                    &FungibleProofOffset::Moveable.into(),
                    LockFlags::MUTABLE,
                    SystemLockData::default(),
                )?;
                let mut proof: ProofMoveableSubstate =
                    api.kernel_read_substate(handle)?.as_typed().unwrap();

                if proof.restricted {
                    return Err(RuntimeError::ModuleError(ModuleError::NodeMoveError(
                        NodeMoveError::CantMoveDownstream(node_id),
                    )));
                }

                if changed_to_restricted {
                    proof.change_to_restricted();
                }

                api.kernel_write_substate(handle, IndexedScryptoValue::from_typed(&proof))?;
                api.kernel_drop_lock(handle)?;
            }
            TypeInfoSubstate::Object(ObjectInfo {
                blueprint,
                outer_object,
                ..
            }) if blueprint.package_address.eq(&RESOURCE_MANAGER_PACKAGE)
                && blueprint.blueprint_name.eq(NON_FUNGIBLE_PROOF_BLUEPRINT) =>
            {
                if matches!(callee, Actor::Method { node_id, .. } if node_id.eq(outer_object.unwrap().as_node_id()))
                {
                    return Ok(());
                }

                if matches!(callee, Actor::Function { .. }) && callee.blueprint().eq(&blueprint) {
                    return Ok(());
                }

                // Change to restricted unless it's moved to auth zone.
                // TODO: align with barrier design?
                let mut changed_to_restricted = true;
                if let Actor::Method { node_id, .. } = callee {
                    let type_info = TypeInfoBlueprint::get_type(node_id, api)?;
                    if let TypeInfoSubstate::Object(ObjectInfo { blueprint, .. }) = type_info {
                        if blueprint.eq(&Blueprint::new(
                            &RESOURCE_MANAGER_PACKAGE,
                            AUTH_ZONE_BLUEPRINT,
                        )) {
                            changed_to_restricted = false;
                        }
                    }
                }

                let handle = api.kernel_lock_substate(
                    &node_id,
                    OBJECT_BASE_MODULE,
                    &NonFungibleProofOffset::Moveable.into(),
                    LockFlags::MUTABLE,
                    SystemLockData::default(),
                )?;
                let mut proof: ProofMoveableSubstate =
                    api.kernel_read_substate(handle)?.as_typed().unwrap();

                if proof.restricted {
                    return Err(RuntimeError::ModuleError(ModuleError::NodeMoveError(
                        NodeMoveError::CantMoveDownstream(node_id),
                    )));
                }

                if changed_to_restricted {
                    proof.change_to_restricted();
                }

                api.kernel_write_substate(handle, IndexedScryptoValue::from_typed(&proof))?;
                api.kernel_drop_lock(handle)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn prepare_move_upstream<Y: KernelApi<M>, M: KernelCallbackObject>(
        _node_id: NodeId,
        _api: &mut Y,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
}

impl<V: SystemCallbackObject> SystemModule<SystemConfig<V>> for NodeMoveModule {
    fn before_push_frame<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        callee: &Actor,
        call_frame_update: &mut CallFrameUpdate,
        _args: &IndexedScryptoValue,
    ) -> Result<(), RuntimeError> {
        for node_id in &call_frame_update.nodes_to_move {
            // TODO: Move into system layer
            Self::prepare_move_downstream(*node_id, callee, api)?;
        }

        Ok(())
    }

    fn on_execution_finish<Y: KernelApi<SystemConfig<V>>>(
        api: &mut Y,
        _caller: &Option<Actor>,
        call_frame_update: &CallFrameUpdate,
    ) -> Result<(), RuntimeError> {
        for node_id in &call_frame_update.nodes_to_move {
            Self::prepare_move_upstream(*node_id, api)?;
        }

        Ok(())
    }
}
