use crate::blueprints::resource::{LocalRef, ProofError, ProofMoveableSubstate};
use crate::errors::{ApplicationError, RuntimeError};
use crate::types::*;
use radix_engine_interface::api::field_api::LockFlags;
use radix_engine_interface::api::{ClientApi, FieldValue, OBJECT_HANDLE_SELF};
use radix_engine_interface::blueprints::resource::*;

#[derive(Debug, Clone, ScryptoSbor)]
pub struct NonFungibleProofSubstate {
    /// The total locked amount or non-fungible ids.
    pub total_locked: BTreeSet<NonFungibleLocalId>,
    /// The supporting containers.
    pub evidence: BTreeMap<LocalRef, BTreeSet<NonFungibleLocalId>>,
}

impl NonFungibleProofSubstate {
    pub fn new(
        total_locked: BTreeSet<NonFungibleLocalId>,
        evidence: BTreeMap<LocalRef, BTreeSet<NonFungibleLocalId>>,
    ) -> Result<NonFungibleProofSubstate, ProofError> {
        if total_locked.is_empty() {
            return Err(ProofError::EmptyProofNotAllowed);
        }

        Ok(Self {
            total_locked,
            evidence,
        })
    }

    pub fn clone_proof<Y: ClientApi<RuntimeError>>(
        &self,
        api: &mut Y,
    ) -> Result<Self, RuntimeError> {
        for (container, locked_ids) in &self.evidence {
            api.call_method(
                container.as_node_id(),
                match container {
                    LocalRef::Bucket(_) => NON_FUNGIBLE_BUCKET_LOCK_NON_FUNGIBLES_IDENT,
                    LocalRef::Vault(_) => NON_FUNGIBLE_VAULT_LOCK_NON_FUNGIBLES_IDENT,
                },
                scrypto_args!(locked_ids),
            )?;
        }
        Ok(Self {
            total_locked: self.total_locked.clone(),
            evidence: self.evidence.clone(),
        })
    }

    pub fn teardown<Y: ClientApi<RuntimeError>>(self, api: &mut Y) -> Result<(), RuntimeError> {
        for (container, locked_ids) in &self.evidence {
            api.call_method(
                container.as_node_id(),
                match container {
                    LocalRef::Bucket(_) => NON_FUNGIBLE_BUCKET_UNLOCK_NON_FUNGIBLES_IDENT,
                    LocalRef::Vault(_) => NON_FUNGIBLE_VAULT_UNLOCK_NON_FUNGIBLES_IDENT,
                },
                scrypto_args!(locked_ids),
            )?;
        }
        Ok(())
    }

    pub fn amount(&self) -> Decimal {
        self.non_fungible_local_ids().len().into()
    }

    pub fn non_fungible_local_ids(&self) -> &BTreeSet<NonFungibleLocalId> {
        &self.total_locked
    }
}

pub struct NonFungibleProofBlueprint;

impl NonFungibleProofBlueprint {
    pub(crate) fn clone<Y>(api: &mut Y) -> Result<Proof, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let moveable = {
            let handle = api.actor_open_field(
                OBJECT_HANDLE_SELF,
                NonFungibleProofField::Moveable,
                LockFlags::read_only(),
            )?;
            let substate_ref: ProofMoveableSubstate = api.field_read_typed(handle)?;
            let moveable = substate_ref.clone();
            api.field_close(handle)?;
            moveable
        };
        let handle = api.actor_open_field(
            OBJECT_HANDLE_SELF,
            NonFungibleProofField::ProofRefs,
            LockFlags::read_only(),
        )?;
        let substate_ref: NonFungibleProofSubstate = api.field_read_typed(handle)?;
        let proof = substate_ref.clone();
        let clone = proof.clone_proof(api)?;

        let proof_id = api.new_simple_object(
            NON_FUNGIBLE_PROOF_BLUEPRINT,
            vec![FieldValue::new(&moveable), FieldValue::new(&clone)],
        )?;

        // Drop after object creation to keep the reference alive
        api.field_close(handle)?;

        Ok(Proof(Own(proof_id)))
    }

    pub(crate) fn get_amount<Y>(api: &mut Y) -> Result<Decimal, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let handle = api.actor_open_field(
            OBJECT_HANDLE_SELF,
            NonFungibleProofField::ProofRefs,
            LockFlags::read_only(),
        )?;
        let substate_ref: NonFungibleProofSubstate = api.field_read_typed(handle)?;
        let amount = substate_ref.amount();
        api.field_close(handle)?;
        Ok(amount)
    }

    pub(crate) fn get_local_ids<Y>(
        api: &mut Y,
    ) -> Result<BTreeSet<NonFungibleLocalId>, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let handle = api.actor_open_field(
            OBJECT_HANDLE_SELF,
            NonFungibleProofField::ProofRefs,
            LockFlags::read_only(),
        )?;
        let substate_ref: NonFungibleProofSubstate = api.field_read_typed(handle)?;
        let ids = substate_ref.non_fungible_local_ids().clone();
        api.field_close(handle)?;
        Ok(ids)
    }

    pub(crate) fn get_resource_address<Y>(api: &mut Y) -> Result<ResourceAddress, RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let address = ResourceAddress::new_or_panic(api.actor_get_outer_object().unwrap().into());
        Ok(address)
    }

    pub(crate) fn drop<Y>(proof: Proof, api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        api.drop_object(proof.0.as_node_id())?;

        Ok(())
    }

    pub(crate) fn on_drop<Y>(api: &mut Y) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        let handle = api.actor_open_field(
            OBJECT_HANDLE_SELF,
            NonFungibleProofField::ProofRefs,
            LockFlags::MUTABLE,
        )?;
        let proof_substate: NonFungibleProofSubstate = api.field_read_typed(handle)?;
        proof_substate.teardown(api)?;
        api.field_close(handle)?;

        Ok(())
    }

    pub(crate) fn on_move<Y>(
        is_moving_down: bool,
        is_to_barrier: bool,
        destination_blueprint_id: Option<BlueprintId>,
        api: &mut Y,
    ) -> Result<(), RuntimeError>
    where
        Y: ClientApi<RuntimeError>,
    {
        if is_moving_down {
            let is_to_self = destination_blueprint_id.eq(&Some(BlueprintId::new(
                &RESOURCE_PACKAGE,
                NON_FUNGIBLE_PROOF_BLUEPRINT,
            )));
            let is_to_auth_zone = destination_blueprint_id.eq(&Some(BlueprintId::new(
                &RESOURCE_PACKAGE,
                AUTH_ZONE_BLUEPRINT,
            )));
            if !is_to_self && (is_to_barrier || is_to_auth_zone) {
                let handle = api.actor_open_field(
                    OBJECT_HANDLE_SELF,
                    FungibleProofField::Moveable,
                    LockFlags::MUTABLE,
                )?;
                let mut proof: ProofMoveableSubstate = api.field_read_typed(handle)?;

                // Check if the proof is restricted
                if proof.restricted {
                    return Err(RuntimeError::ApplicationError(ApplicationError::Panic(
                        "Moving restricted proof downstream".to_owned(),
                    )));
                }

                // Update restricted flag
                if is_to_barrier {
                    proof.change_to_restricted();
                }

                api.field_write_typed(handle, &proof)?;
                api.field_close(handle)?;
                Ok(())
            } else {
                // Proofs can move freely as long as it's not to a barrier or auth zone.
                Ok(())
            }
        } else {
            // No restriction for moving up
            Ok(())
        }
    }
}
