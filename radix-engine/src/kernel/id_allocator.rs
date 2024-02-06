use crate::errors::{IdAllocationError, KernelError, RuntimeError};
use crate::types::*;

/// An ID allocator defines how identities are generated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdAllocator {
    transaction_hash: Hash,
    next_ids: [u32; 256],
}

impl IdAllocator {
    pub fn new(transaction_hash: Hash, next_ids: [u32; 256]) -> Self {
        Self {
            transaction_hash,
            next_ids
        }
    }

    pub fn allocate_node_id(&mut self, entity_type: EntityType) -> Result<NodeId, RuntimeError> {
        let node_id = self
            .next_node_id(entity_type)
            .map_err(|e| RuntimeError::KernelError(KernelError::IdAllocationError(e)))?;

        Ok(node_id)
    }

    pub fn get_next_node_ids(&self) -> [u32; 256] {
        self.next_ids.clone()
    }

    fn next(&mut self, entity_type: EntityType) -> Result<u32, IdAllocationError> {
        let next_id = &mut self.next_ids[entity_type as usize];
        if *next_id >= u16::MAX as u32 {
            Err(IdAllocationError::OutOfID)
        } else {
            let rtn = *next_id;
            *next_id += 1;
            Ok(rtn)
        }
    }

    fn next_node_id(&mut self, entity_type: EntityType) -> Result<NodeId, IdAllocationError> {
        // Install the entity type
        let next_id = self.next(entity_type)?;
        let mut node_id: [u8; NodeId::LENGTH] = [0; NodeId::LENGTH];
        node_id[0] = entity_type as u8;
        node_id[1] = ((next_id >> 8) & 0xFF) as u8;
        node_id[2] = (next_id & 0xFF) as u8;
        Ok(NodeId(node_id))
    }
}
