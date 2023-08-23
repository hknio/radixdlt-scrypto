use crate::api::ObjectHandle;
use radix_engine_common::data::scrypto::{
    scrypto_decode, scrypto_encode, ScryptoDecode, ScryptoEncode,
};
use radix_engine_interface::api::*;
use sbor::rust::prelude::*;
use sbor::rust::vec::Vec;

pub trait IndexKeyPayloadMarker {}
pub trait IndexEntryPayloadMarker {}

/// Api to manage an iterable index
pub trait ClientActorIndexApi<E> {
    /// Inserts an entry into an index
    fn actor_index_insert(
        &mut self,
        object_handle: ObjectHandle,
        collection_index: impl CollectionDescriptor,
        key: Vec<u8>,
        buffer: Vec<u8>,
    ) -> Result<(), E>;

    /// Inserts an entry into an index
    fn actor_index_insert_typed<
        // TODO: add a IndexKeyPayloadMarker bound once all native blueprints have been updated
        K: ScryptoEncode,
        // TODO: add a IndexEntryPayloadMarker bound once all native blueprints have been updated
        V: ScryptoEncode,
    >(
        &mut self,
        object_handle: ObjectHandle,
        collection_index: impl CollectionDescriptor,
        key: K,
        value: V,
    ) -> Result<(), E> {
        self.actor_index_insert(
            object_handle,
            collection_index,
            scrypto_encode(&key).unwrap(),
            scrypto_encode(&value).unwrap(),
        )
    }

    /// Removes an entry from an index
    fn actor_index_remove(
        &mut self,
        object_handle: ObjectHandle,
        collection_index: impl CollectionDescriptor,
        key: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, E>;

    /// Removes an entry from an index
    fn actor_index_remove_typed<
        // TODO: add a IndexEntryPayloadMarker bound once all native blueprints have been updated
        V: ScryptoDecode,
    >(
        &mut self,
        object_handle: ObjectHandle,
        collection_index: impl CollectionDescriptor,
        key: Vec<u8>,
    ) -> Result<Option<V>, E> {
        let rtn = self
            .actor_index_remove(object_handle, collection_index, key)?
            .map(|e| scrypto_decode(&e).unwrap());
        Ok(rtn)
    }

    /// Scans arbitrary elements of count from an index
    fn actor_index_scan_keys(
        &mut self,
        object_handle: ObjectHandle,
        collection_index: impl CollectionDescriptor,
        limit: u32,
    ) -> Result<Vec<Vec<u8>>, E>;

    /// Scans arbitrary elements of count from an index
    fn actor_index_scan_keys_typed<
        // TODO: add a IndexKeyPayloadMarker bound once all native blueprints have been updated
        K: ScryptoDecode,
    >(
        &mut self,
        object_handle: ObjectHandle,
        collection_index: impl CollectionDescriptor,
        limit: u32,
    ) -> Result<Vec<K>, E> {
        let entries = self
            .actor_index_scan_keys(object_handle, collection_index, limit)?
            .into_iter()
            .map(|key| {
                let key: K = scrypto_decode(&key).unwrap();
                key
            })
            .collect();

        Ok(entries)
    }

    /// Removes and returns arbitrary elements of count from an index
    fn actor_index_drain(
        &mut self,
        object_handle: ObjectHandle,
        collection_index: impl CollectionDescriptor,
        limit: u32,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, E>;

    /// Removes and returns arbitrary elements of count from an index
    fn actor_index_drain_typed<
        // TODO: add a IndexKeyPayloadMarker bound once all native blueprints have been updated
        K: ScryptoDecode,
        // TODO: add a IndexEntryPayloadMarker bound once all native blueprints have been updated
        V: ScryptoDecode,
    >(
        &mut self,
        object_handle: ObjectHandle,
        collection_index: impl CollectionDescriptor,
        limit: u32,
    ) -> Result<Vec<(K, V)>, E> {
        let entries = self
            .actor_index_drain(object_handle, collection_index, limit)?
            .into_iter()
            .map(|(key, value)| {
                let key: K = scrypto_decode(&key).unwrap();
                let value: V = scrypto_decode(&value).unwrap();
                (key, value)
            })
            .collect();

        Ok(entries)
    }
}
