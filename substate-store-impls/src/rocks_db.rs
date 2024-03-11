use itertools::Itertools;
use radix_engine_common::constants::MAX_SUBSTATE_KEY_SIZE;
pub use rocksdb::{BlockBasedOptions, LogLevel, Options};
use rocksdb::{
    ColumnFamily, ColumnFamilyDescriptor, DBWithThreadMode, Direction, IteratorMode,
    SingleThreaded, DB,
};
use sbor::rust::prelude::*;
use std::path::PathBuf;
use substate_store_interface::interface::*;
use utils::copy_u8_array;

pub struct RocksdbSubstateStore {
    db: DBWithThreadMode<SingleThreaded>,
}

impl RocksdbSubstateStore {
    const SUBSTATES_CF: &'static str = "substates";

    pub fn standard(root: PathBuf) -> Self {
        Self::with_options(&Options::default(), root)
    }

    pub fn with_options(options: &Options, root: PathBuf) -> Self {
        let cfs: Vec<ColumnFamilyDescriptor> = match DB::list_cf(&Options::default(), root.as_path()) {
            Ok(cfs) => {
                cfs.iter().map(|cf_name| {
                    ColumnFamilyDescriptor::new(cf_name, Options::default())
                }).collect()
            },
            Err(_) => {
                vec![ColumnFamilyDescriptor::new(
                    Self::SUBSTATES_CF,
                    Options::default(),
                )]                
            }
        };

        let mut options = options.clone();
        options.create_if_missing(true);
        options.create_missing_column_families(true);
        let db = DB::open_cf_descriptors(
            &options,
            root.as_path(),
            cfs
        )
        .unwrap();
        Self { db }
    }

    fn cf(&self) -> &ColumnFamily {
        self.db.cf_handle(Self::SUBSTATES_CF).unwrap()
    }
}

impl SubstateDatabase for RocksdbSubstateStore {
    fn get_substate(
        &self,
        partition_key: &DbPartitionKey,
        sort_key: &DbSortKey,
    ) -> Option<DbSubstateValue> {
        let key_bytes = encode_to_rocksdb_bytes(partition_key, sort_key);
        self.db.get_cf(self.cf(), &key_bytes).expect("IO Error")
    }

    fn list_entries_from(
        &self,
        partition_key: &DbPartitionKey,
        from_sort_key: Option<&DbSortKey>,
    ) -> Box<dyn Iterator<Item = PartitionEntry> + '_> {
        let partition_key = partition_key.clone();
        let empty_sort_key = DbSortKey(vec![]);
        let from_sort_key = from_sort_key.unwrap_or(&empty_sort_key);
        let start_key_bytes = encode_to_rocksdb_bytes(&partition_key, from_sort_key);
        let iter = self
            .db
            .iterator_cf(
                self.cf(),
                IteratorMode::From(&start_key_bytes, Direction::Forward),
            )
            .map(|kv| {
                let (iter_key_bytes, iter_value) = kv.as_ref().unwrap();
                let iter_key = decode_from_rocksdb_bytes(iter_key_bytes);
                (iter_key, iter_value.to_vec())
            })
            .take_while(move |((iter_partition_key, _), _)| *iter_partition_key == partition_key)
            .map(|((_, iter_sort_key), iter_value)| (iter_sort_key, iter_value.to_vec()));

        Box::new(iter)
    }
}

impl CommittableSubstateDatabase for RocksdbSubstateStore {
    fn commit(&mut self, database_updates: &DatabaseUpdates) {
        for (node_key, node_updates) in &database_updates.node_updates {
            for (partition_num, partition_updates) in &node_updates.partition_updates {
                let partition_key = DbPartitionKey {
                    node_key: node_key.clone(),
                    partition_num: *partition_num,
                };
                match partition_updates {
                    PartitionDatabaseUpdates::Delta { substate_updates } => {
                        for (sort_key, update) in substate_updates {
                            let key_bytes = encode_to_rocksdb_bytes(&partition_key, sort_key);
                            match update {
                                DatabaseUpdate::Set(value_bytes) => {
                                    self.db.put_cf(self.cf(), key_bytes, value_bytes)
                                }
                                DatabaseUpdate::Delete => self.db.delete_cf(self.cf(), key_bytes),
                            }
                            .expect("IO error");
                        }
                    }
                    PartitionDatabaseUpdates::Reset {
                        new_substate_values,
                    } => {
                        // Note: a plain `delete_range()` is missing from rocksdb's API, and
                        // (at the moment of writing) this is the only reason of having CF.
                        self.db
                            .delete_range_cf(
                                self.cf(),
                                encode_to_rocksdb_bytes(&partition_key, &DbSortKey(vec![])),
                                encode_to_rocksdb_bytes(
                                    &partition_key,
                                    &DbSortKey(vec![u8::MAX; 2 * MAX_SUBSTATE_KEY_SIZE]),
                                ),
                            )
                            .expect("IO error");
                        for (sort_key, value_bytes) in new_substate_values {
                            let key_bytes = encode_to_rocksdb_bytes(&partition_key, sort_key);
                            self.db
                                .put_cf(self.cf(), key_bytes, value_bytes)
                                .expect("IO error");
                        }
                    }
                }
            }
        }
    }
}

impl ListableSubstateDatabase for RocksdbSubstateStore {
    fn list_partition_keys(&self) -> Box<dyn Iterator<Item = DbPartitionKey> + '_> {
        Box::new(
            self.db
                .iterator_cf(self.cf(), IteratorMode::Start)
                .map(|kv| {
                    let (iter_key_bytes, _) = kv.as_ref().unwrap();
                    let (iter_key, _) = decode_from_rocksdb_bytes(iter_key_bytes);
                    iter_key
                })
                // Rocksdb iterator returns sorted entries, so ok to to eliminate
                // duplicates with dedup()
                .dedup(),
        )
    }
}

pub fn encode_to_rocksdb_bytes(partition_key: &DbPartitionKey, sort_key: &DbSortKey) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(1 + partition_key.node_key.len() + 1 + sort_key.0.len());
    buffer.push(
        u8::try_from(partition_key.node_key.len())
            .expect("Node key length is effectively constant 32 so should fit in a u8"),
    );
    buffer.extend(partition_key.node_key.clone());
    buffer.push(partition_key.partition_num);
    buffer.extend(sort_key.0.clone());
    buffer
}

pub fn decode_from_rocksdb_bytes(buffer: &[u8]) -> DbSubstateKey {
    let node_key_start: usize = 1usize;
    let partition_key_start = 1usize + usize::from(buffer[0]);
    let sort_key_start = 1usize + partition_key_start;
    let node_key = buffer[node_key_start..partition_key_start].to_vec();
    let partition_num = buffer[partition_key_start];
    let sort_key = buffer[sort_key_start..].to_vec();
    (
        DbPartitionKey {
            node_key,
            partition_num,
        },
        DbSortKey(sort_key),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use substate_store_interface::interface::{
        CommittableSubstateDatabase, DatabaseUpdates, DbSortKey, NodeDatabaseUpdates,
        PartitionDatabaseUpdates,
    };

    #[cfg(not(feature = "alloc"))]
    #[test]
    fn test_partition_deletion() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut db = RocksdbSubstateStore::standard(temp_dir.into_path());

        let node_updates = NodeDatabaseUpdates {
            partition_updates: indexmap! {
                0 => PartitionDatabaseUpdates::Reset {
                    new_substate_values: indexmap! {
                        DbSortKey(vec![5]) => vec![6]
                    }
                },
                1 => PartitionDatabaseUpdates::Reset {
                    new_substate_values: indexmap! {
                        DbSortKey(vec![7]) => vec![8]
                    }
                },
                255 => PartitionDatabaseUpdates::Reset {
                    new_substate_values: indexmap! {
                        DbSortKey(vec![9]) => vec![10]
                    }
                }
            },
        };
        let updates = DatabaseUpdates {
            node_updates: indexmap! {
                vec![0] => node_updates.clone(),
                vec![1] => node_updates.clone(),
                vec![255] => node_updates.clone(),
            },
        };
        db.commit(&updates);

        assert_eq!(db.list_partition_keys().count(), 9);
        db.commit(&DatabaseUpdates {
            node_updates: indexmap! {
                vec![0] => NodeDatabaseUpdates {
                    partition_updates: indexmap!{
                        255 => PartitionDatabaseUpdates::Reset { new_substate_values: indexmap!{} }
                    }
                }
            },
        });
        assert_eq!(db.list_partition_keys().count(), 8);
    }
}
