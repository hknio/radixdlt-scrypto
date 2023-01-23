use crate::engine::{deref_and_update, ApplicationError, Executor, LockFlags, RENode};
use crate::engine::{
    CallFrameUpdate, ExecutableInvocation, ResolvedActor, ResolverApi, RuntimeError, SystemApi,
};
use crate::model::{AccessRulesChainSubstate, GlobalAddressSubstate};
use crate::types::{HashMap, Vec};
use crate::wasm::WasmEngine;
use native_sdk::resource::{SysBucket, Vault};
use native_sdk::runtime::Runtime;
use radix_engine_interface::api::types::*;
use radix_engine_interface::constants::{CLOCK, PACKAGE_TOKEN};
use radix_engine_interface::data::scrypto_encode;
use radix_engine_interface::*;
use radix_engine_interface::{api::*, rule};

use super::AccessControllerSubstate;

#[derive(Debug, Clone, PartialEq, Eq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub enum AccessControllerError {
    OperationNotAllowedWhenPrimaryIsLocked,
    RecoveryForThisProposerAlreadyExists { proposer: Proposer },
    NoValidProposedRuleSetExists,
    TimeOverflow,
    TimedRecoveryDelayHasNotElapsed,
}

impl From<AccessControllerError> for RuntimeError {
    fn from(value: AccessControllerError) -> Self {
        RuntimeError::ApplicationError(ApplicationError::AccessControllerError(value))
    }
}

//=================================
// Access Controller Create Global
//=================================

impl ExecutableInvocation for AccessControllerCreateGlobalInvocation {
    type Exec = Self;

    fn resolve<D: ResolverApi>(
        self,
        _deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let actor =
            ResolvedActor::function(NativeFn::AccessController(AccessControllerFn::CreateGlobal));
        let call_frame_update = CallFrameUpdate::move_node(RENodeId::Bucket(self.controlled_asset));

        Ok((actor, call_frame_update, self))
    }
}

impl Executor for AccessControllerCreateGlobalInvocation {
    type Output = ComponentAddress;

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        // Creating a new vault and putting in it the controlled asset
        let vault = {
            let controlled_asset = Bucket(self.controlled_asset);

            let mut vault = controlled_asset
                .sys_resource_address(api)
                .and_then(|resource_address| Vault::sys_new(resource_address, api))?;
            vault.sys_put(controlled_asset, api)?;

            vault
        };

        // Creating the Access Controller substate
        let access_controller_substate = AccessControllerSubstate {
            controlled_asset: vault.0,
            ongoing_recoveries: None,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
            is_primary_role_locked: false,
        };
        let access_rules_chain_substate = AccessRulesChainSubstate {
            access_rules_chain: [access_rules_from_rule_set(self.rule_set)].into(),
        };

        // Allocating an RENodeId and creating the access controller RENode
        let node_id = api.allocate_node_id(RENodeType::AccessController)?;
        api.create_node(
            node_id,
            RENode::AccessController(access_controller_substate, access_rules_chain_substate),
        )?;

        // Creating a global component address for the access controller RENode
        let global_node_id = api.allocate_node_id(RENodeType::GlobalAccessController)?;
        api.create_node(
            global_node_id,
            RENode::Global(GlobalAddressSubstate::AccessController(node_id.into())),
        )?;

        Ok((global_node_id.into(), CallFrameUpdate::empty()))
    }
}

//================================
// Access Controller Create Proof
//================================

#[derive(Debug, Clone, Eq, PartialEq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct AccessControllerCreateProofExecutable {
    pub receiver: RENodeId,
}

impl ExecutableInvocation for AccessControllerCreateProofInvocation {
    type Exec = AccessControllerCreateProofExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::CreateProof),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for AccessControllerCreateProofExecutable {
    type Output = Proof;

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        // Access Controller Substate Handle
        let node_id = self.receiver;
        let offset = SubstateOffset::AccessController(AccessControllerOffset::AccessController);
        let handle = api.lock_substate(node_id, offset, LockFlags::read_only())?;

        let substate = api.get_ref(handle)?;
        let access_controller = substate.access_controller();

        if access_controller.is_primary_role_locked {
            Err(AccessControllerError::OperationNotAllowedWhenPrimaryIsLocked)?
        }

        // Creating a proof of the controlled asset
        let proof = {
            let substate = api.get_ref(handle)?;
            let access_controller = substate.access_controller();

            Vault(access_controller.controlled_asset).sys_create_proof(api)?
        };

        let call_frame_update = CallFrameUpdate::move_node(RENodeId::Proof(proof.0));
        api.drop_lock(handle)?;

        Ok((proof, call_frame_update))
    }
}

//=====================================
// Access Controller Initiate Recovery
//=====================================

#[derive(Debug, Clone, Eq, PartialEq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct AccessControllerInitiateRecoveryExecutable {
    pub receiver: RENodeId,
    pub rule_set: RuleSet,
    pub proposer: Proposer,
    pub timed_recovery_delay_in_hours: u16,
}

impl ExecutableInvocation for AccessControllerInitiateRecoveryAsPrimaryInvocation {
    type Exec = AccessControllerInitiateRecoveryExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        call_frame_update.add_ref(RENodeId::Global(GlobalAddress::Component(CLOCK)));
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::InitiateRecoveryAsPrimary),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
            rule_set: self.rule_set,
            proposer: Proposer::Primary,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl ExecutableInvocation for AccessControllerInitiateRecoveryAsRecoveryInvocation {
    type Exec = AccessControllerInitiateRecoveryExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        call_frame_update.add_ref(RENodeId::Global(GlobalAddress::Component(CLOCK)));
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::InitiateRecoveryAsRecovery),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
            rule_set: self.rule_set,
            proposer: Proposer::Recovery,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for AccessControllerInitiateRecoveryExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        // Access Controller Substate Handle
        let node_id = self.receiver;
        let offset = SubstateOffset::AccessController(AccessControllerOffset::AccessController);
        let handle = api.lock_substate(node_id, offset, LockFlags::MUTABLE)?;

        // Lock checks and Getting the timed recovery delay
        let timed_recovery_delay_in_hours = {
            let substate = api.get_ref(handle)?;
            let access_controller = substate.access_controller();

            if self.proposer == Proposer::Primary && access_controller.is_primary_role_locked {
                Err(AccessControllerError::OperationNotAllowedWhenPrimaryIsLocked)?
            }

            access_controller.timed_recovery_delay_in_hours
        };

        // Getting the time when timed recovery will be allowed
        let timed_recovery_end_time = Runtime::sys_current_time(api, TimePrecision::Minute)?
            .add_hours(timed_recovery_delay_in_hours as i64)
            .map_or(
                Err(RuntimeError::from(AccessControllerError::TimeOverflow)),
                |instant| Ok(instant),
            )?;

        // Initiate Recovery (if this role doesn't already have a recovery Ongoing)
        {
            let mut substate = api.get_ref_mut(handle)?;
            let access_controller = substate.access_controller();

            let recoveries = match access_controller.ongoing_recoveries.as_mut() {
                Some(ongoing_recoveries) => ongoing_recoveries,
                None => {
                    access_controller.ongoing_recoveries = Some(HashMap::new());
                    access_controller
                        .ongoing_recoveries
                        .as_mut()
                        .expect("Impossible Case!")
                }
            };

            if !recoveries.contains_key(&self.proposer) {
                recoveries.insert(
                    self.proposer,
                    (
                        self.rule_set,
                        self.timed_recovery_delay_in_hours,
                        timed_recovery_end_time,
                    ),
                );
            } else {
                Err(
                    AccessControllerError::RecoveryForThisProposerAlreadyExists {
                        proposer: self.proposer,
                    },
                )?;
            }
        }

        api.drop_lock(handle)?;

        Ok(((), CallFrameUpdate::empty()))
    }
}

//==========================================
// Access Controller Quick Confirm Recovery
//==========================================

#[derive(Debug, Clone, Eq, PartialEq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct AccessControllerQuickConfirmRecoveryExecutable {
    pub receiver: RENodeId,
    pub rule_set: RuleSet,
    pub proposer: Proposer,
    pub confirmor: Role,
    pub timed_recovery_delay_in_hours: u16,
}

impl ExecutableInvocation for AccessControllerQuickConfirmRecoveryAsPrimaryInvocation {
    type Exec = AccessControllerQuickConfirmRecoveryExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::QuickConfirmRecoveryAsPrimary),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
            rule_set: self.rule_set,
            proposer: self.proposer,
            confirmor: Role::Primary,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl ExecutableInvocation for AccessControllerQuickConfirmRecoveryAsRecoveryInvocation {
    type Exec = AccessControllerQuickConfirmRecoveryExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::QuickConfirmRecoveryAsRecovery),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
            rule_set: self.rule_set,
            proposer: self.proposer,
            confirmor: Role::Recovery,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl ExecutableInvocation for AccessControllerQuickConfirmRecoveryAsConfirmationInvocation {
    type Exec = AccessControllerQuickConfirmRecoveryExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::QuickConfirmRecoveryAsConfirmation),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
            rule_set: self.rule_set,
            proposer: self.proposer,
            confirmor: Role::Confirmation,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for AccessControllerQuickConfirmRecoveryExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        // Access Controller Substate Handle
        let node_id = self.receiver;
        let offset = SubstateOffset::AccessController(AccessControllerOffset::AccessController);
        let handle = api.lock_substate(node_id, offset, LockFlags::MUTABLE)?;

        // Quick confirm and get new rule set
        let (new_rule_set, timed_recovery_delay_in_hours) = {
            let substate = api.get_ref(handle)?;
            let access_controller = substate.access_controller();

            access_controller
                .ongoing_recoveries
                .as_ref()
                .unwrap_or(&HashMap::new())
                .iter()
                .find(
                    |(proposer, (proposed_rule_set, timed_recovery_delay_in_hours, _))| {
                        **proposer == self.proposer
                            && *proposed_rule_set == self.rule_set
                            && Role::from(self.proposer) != self.confirmor
                            && *timed_recovery_delay_in_hours == self.timed_recovery_delay_in_hours
                    },
                )
                .map_or(
                    Err(AccessControllerError::NoValidProposedRuleSetExists),
                    |(_, (rule_set, timed_recovery_delay_in_hours, _))| {
                        Ok((rule_set.clone(), *timed_recovery_delay_in_hours))
                    },
                )?
        };

        // Update the access rules substate
        // TODO: We need something better here. Update once we have an AccessRulesChain invocation
        //       for bulk updates.
        {
            let node_id = self.receiver;
            let offset = SubstateOffset::AccessRulesChain(AccessRulesChainOffset::AccessRulesChain);
            let handle = api.lock_substate(node_id, offset, LockFlags::MUTABLE)?;

            let mut substate = api.get_ref_mut(handle)?;
            let access_rule_chain = substate.access_rules_chain();

            access_rule_chain.access_rules_chain =
                [access_rules_from_rule_set(new_rule_set)].into();

            api.drop_lock(handle)?;
        }

        // Unlock Primary and set the timed recovery delay
        {
            let mut substate = api.get_ref_mut(handle)?;
            let access_controller = substate.access_controller();
            access_controller.is_primary_role_locked = false;
            access_controller.timed_recovery_delay_in_hours = timed_recovery_delay_in_hours;
        }

        api.drop_lock(handle)?;

        Ok(((), CallFrameUpdate::empty()))
    }
}

//==========================================
// Access Controller Timed Confirm Recovery
//==========================================

#[derive(Debug, Clone, Eq, PartialEq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct AccessControllerTimedConfirmRecoveryExecutable {
    pub receiver: RENodeId,
    pub rule_set: RuleSet,
    pub proposer: Proposer,
    pub timed_recovery_delay_in_hours: u16,
}

impl ExecutableInvocation for AccessControllerTimedConfirmRecoveryAsPrimaryInvocation {
    type Exec = AccessControllerTimedConfirmRecoveryExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        call_frame_update.add_ref(RENodeId::Global(GlobalAddress::Component(CLOCK)));
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::TimedConfirmRecoveryAsPrimary),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
            rule_set: self.rule_set,
            proposer: Proposer::Primary,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl ExecutableInvocation for AccessControllerTimedConfirmRecoveryAsRecoveryInvocation {
    type Exec = AccessControllerTimedConfirmRecoveryExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        call_frame_update.add_ref(RENodeId::Global(GlobalAddress::Component(CLOCK)));
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::TimedConfirmRecoveryAsRecovery),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
            rule_set: self.rule_set,
            proposer: Proposer::Recovery,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for AccessControllerTimedConfirmRecoveryExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        // Access Controller Substate Handle
        let node_id = self.receiver;
        let offset = SubstateOffset::AccessController(AccessControllerOffset::AccessController);
        let handle = api.lock_substate(node_id, offset, LockFlags::MUTABLE)?;

        // Getting the RuleSet (if exists) of the new active role
        let (new_rule_set, timed_recovery_delay_in_hours) = {
            let substate = api.get_ref(handle)?;
            let access_controller = substate.access_controller();

            let (new_rule_set, timed_recovery_delay_in_hours, timed_recovery_end_time) =
                access_controller
                    .ongoing_recoveries
                    .as_ref()
                    .unwrap_or(&HashMap::new())
                    .iter()
                    .find(
                        |(proposer, (proposed_rule_set, timed_recovery_delay_in_hours, _))| {
                            **proposer == self.proposer
                                && *proposed_rule_set == self.rule_set
                                && *timed_recovery_delay_in_hours
                                    == self.timed_recovery_delay_in_hours
                        },
                    )
                    .map_or(
                        Err(AccessControllerError::NoValidProposedRuleSetExists),
                        |(
                            _,
                            (rule_set, timed_recovery_delay_in_hours, timed_recovery_end_time),
                        )| {
                            Ok((
                                rule_set.clone(),
                                *timed_recovery_delay_in_hours,
                                timed_recovery_end_time.clone(),
                            ))
                        },
                    )?;
            let recovery_time_has_elapsed = Runtime::sys_compare_against_current_time(
                api,
                timed_recovery_end_time,
                TimePrecision::Minute,
                time::TimeComparisonOperator::Gte,
            )?;

            if !recovery_time_has_elapsed {
                Err(AccessControllerError::TimedRecoveryDelayHasNotElapsed)?
            }

            (new_rule_set, timed_recovery_delay_in_hours)
        };

        // Update the access rules substate
        // TODO: We need something better here. Update once we have an AccessRulesChain invocation
        //       for bulk updates.
        {
            let node_id = self.receiver;
            let offset = SubstateOffset::AccessRulesChain(AccessRulesChainOffset::AccessRulesChain);
            let handle = api.lock_substate(node_id, offset, LockFlags::MUTABLE)?;

            let mut substate = api.get_ref_mut(handle)?;
            let access_rule_chain = substate.access_rules_chain();

            access_rule_chain.access_rules_chain =
                [access_rules_from_rule_set(new_rule_set)].into();

            api.drop_lock(handle)?;
        }

        // Unlock primary
        {
            let mut substate = api.get_ref_mut(handle)?;
            let access_controller = substate.access_controller();
            access_controller.is_primary_role_locked = false;
            access_controller.timed_recovery_delay_in_hours = timed_recovery_delay_in_hours;
        }

        api.drop_lock(handle)?;

        Ok(((), CallFrameUpdate::empty()))
    }
}

//===========================================
// Access Controller Cancel Recovery Attempt
//===========================================

#[derive(Debug, Clone, Eq, PartialEq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct AccessControllerCancelRecoveryAttemptExecutable {
    pub receiver: RENodeId,
    pub rule_set: RuleSet,
    pub proposer: Proposer,
    pub timed_recovery_delay_in_hours: u16,
}

impl ExecutableInvocation for AccessControllerCancelRecoveryAttemptAsPrimaryInvocation {
    type Exec = AccessControllerCancelRecoveryAttemptExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::CancelRecoveryAttemptAsPrimary),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
            rule_set: self.rule_set,
            proposer: Proposer::Primary,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl ExecutableInvocation for AccessControllerCancelRecoveryAttemptAsRecoveryInvocation {
    type Exec = AccessControllerCancelRecoveryAttemptExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::CancelRecoveryAttemptAsRecovery),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
            rule_set: self.rule_set,
            proposer: Proposer::Recovery,
            timed_recovery_delay_in_hours: self.timed_recovery_delay_in_hours,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for AccessControllerCancelRecoveryAttemptExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        // Access Controller Substate Handle
        let node_id = self.receiver;
        let offset = SubstateOffset::AccessController(AccessControllerOffset::AccessController);
        let handle = api.lock_substate(node_id, offset, LockFlags::MUTABLE)?;

        // Removing the proposed rule set
        {
            let mut substate = api.get_ref_mut(handle)?;
            let access_controller = substate.access_controller();

            access_controller
                .ongoing_recoveries
                .as_mut()
                .unwrap_or(&mut HashMap::new())
                .remove_entry(&self.proposer)
                .map_or(
                    Err(RuntimeError::from(
                        AccessControllerError::NoValidProposedRuleSetExists,
                    )),
                    |_| Ok(()),
                )?;
        }

        api.drop_lock(handle)?;

        Ok(((), CallFrameUpdate::empty()))
    }
}

//=====================================
// Access Controller Lock Primary Role
//=====================================

#[derive(Debug, Clone, Eq, PartialEq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct AccessControllerLockPrimaryRoleExecutable {
    pub receiver: RENodeId,
}

impl ExecutableInvocation for AccessControllerLockPrimaryRoleInvocation {
    type Exec = AccessControllerLockPrimaryRoleExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::LockPrimaryRole),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for AccessControllerLockPrimaryRoleExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        // Access Controller Substate Handle
        let node_id = self.receiver;
        let offset = SubstateOffset::AccessController(AccessControllerOffset::AccessController);
        let handle = api.lock_substate(node_id, offset, LockFlags::MUTABLE)?;

        // Lock the primary role
        {
            let mut substate = api.get_ref_mut(handle)?;
            let access_controller = substate.access_controller();

            access_controller.is_primary_role_locked = true
        }

        api.drop_lock(handle)?;

        Ok(((), CallFrameUpdate::empty()))
    }
}

//=======================================
// Access Controller Unlock Primary Role
//=======================================

#[derive(Debug, Clone, Eq, PartialEq, ScryptoCategorize, ScryptoEncode, ScryptoDecode)]
pub struct AccessControllerUnlockPrimaryRoleExecutable {
    pub receiver: RENodeId,
}

impl ExecutableInvocation for AccessControllerUnlockPrimaryRoleInvocation {
    type Exec = AccessControllerUnlockPrimaryRoleExecutable;

    fn resolve<D: ResolverApi>(
        self,
        deref: &mut D,
    ) -> Result<(ResolvedActor, CallFrameUpdate, Self::Exec), RuntimeError>
    where
        Self: Sized,
    {
        let mut call_frame_update = CallFrameUpdate::empty();
        let receiver = RENodeId::Global(GlobalAddress::Component(self.receiver));
        let resolved_receiver = deref_and_update(receiver, &mut call_frame_update, deref)?;

        let actor = ResolvedActor::method(
            NativeFn::AccessController(AccessControllerFn::UnlockPrimaryRole),
            resolved_receiver,
        );

        let executor = Self::Exec {
            receiver: resolved_receiver.receiver,
        };

        Ok((actor, call_frame_update, executor))
    }
}

impl Executor for AccessControllerUnlockPrimaryRoleExecutable {
    type Output = ();

    fn execute<Y, W: WasmEngine>(
        self,
        api: &mut Y,
    ) -> Result<(Self::Output, CallFrameUpdate), RuntimeError>
    where
        Y: SystemApi + EngineApi<RuntimeError> + InvokableModel<RuntimeError>,
    {
        // Access Controller Substate Handle
        let node_id = self.receiver;
        let offset = SubstateOffset::AccessController(AccessControllerOffset::AccessController);
        let handle = api.lock_substate(node_id, offset, LockFlags::MUTABLE)?;

        // Unlock the primary role
        {
            let mut substate = api.get_ref_mut(handle)?;
            let access_controller = substate.access_controller();

            access_controller.is_primary_role_locked = false
        }

        api.drop_lock(handle)?;

        Ok(((), CallFrameUpdate::empty()))
    }
}

//=========
// Helpers
//=========

fn access_rule_or(access_rules: Vec<AccessRule>) -> AccessRule {
    let mut rule_nodes = Vec::new();
    for access_rule in access_rules.into_iter() {
        match access_rule {
            AccessRule::AllowAll => return AccessRule::AllowAll,
            AccessRule::DenyAll => {}
            AccessRule::Protected(rule_node) => rule_nodes.push(rule_node),
        }
    }
    AccessRule::Protected(AccessRuleNode::AnyOf(rule_nodes))
}

fn access_rules_from_rule_set(rule_set: RuleSet) -> AccessRules {
    let mut access_rules = AccessRules::new();

    // Primary Role Rules
    let primary_group = "primary";
    access_rules.set_group_access_rule(primary_group.into(), rule_set.primary_role.clone());
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(AccessControllerFn::CreateProof)),
        primary_group.into(),
    );
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::InitiateRecoveryAsPrimary,
        )),
        primary_group.into(),
    );
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::QuickConfirmRecoveryAsPrimary,
        )),
        primary_group.into(),
    );
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::TimedConfirmRecoveryAsPrimary,
        )),
        primary_group.into(),
    );
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::CancelRecoveryAttemptAsPrimary,
        )),
        primary_group.into(),
    );

    // Recovery Role Rules
    let recovery_group = "recovery";
    access_rules.set_group_access_rule(recovery_group.into(), rule_set.recovery_role.clone());
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::InitiateRecoveryAsRecovery,
        )),
        recovery_group.into(),
    );
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::QuickConfirmRecoveryAsRecovery,
        )),
        recovery_group.into(),
    );
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::TimedConfirmRecoveryAsRecovery,
        )),
        recovery_group.into(),
    );
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::CancelRecoveryAttemptAsRecovery,
        )),
        recovery_group.into(),
    );

    // Confirmation Role Rules
    let confirmation_group = "confirmation";
    access_rules.set_group_access_rule(
        confirmation_group.into(),
        rule_set.confirmation_role.clone(),
    );
    access_rules.set_method_access_rule_to_group(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::QuickConfirmRecoveryAsConfirmation,
        )),
        confirmation_group.into(),
    );

    // Other methods
    access_rules.set_method_access_rule(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::LockPrimaryRole,
        )),
        access_rule_or([rule_set.primary_role, rule_set.recovery_role.clone()].into()),
    );
    access_rules.set_method_access_rule(
        AccessRuleKey::Native(NativeFn::AccessController(
            AccessControllerFn::UnlockPrimaryRole,
        )),
        access_rule_or([rule_set.recovery_role, rule_set.confirmation_role].into()),
    );

    let non_fungible_global_id =
        scrypto_encode(&PackageIdentifier::Native(NativePackage::AccessController))
            .map(|bytes| NonFungibleGlobalId::new(PACKAGE_TOKEN, NonFungibleLocalId::Bytes(bytes)))
            .unwrap();

    access_rules.default(rule!(deny_all), rule!(require(non_fungible_global_id)))
}
