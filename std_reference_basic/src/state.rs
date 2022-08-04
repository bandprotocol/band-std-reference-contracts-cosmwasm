use cosmwasm_std::{CanonicalAddr, Storage};
use cosmwasm_storage::{
    singleton, singleton_read, PrefixedStorage, ReadonlyPrefixedStorage, ReadonlySingleton,
    Singleton,
};

pub static OWNER_KEY: &[u8] = b"owner";
pub static RELAYERS_KEY: &[u8] = b"relayers";
pub static REFS_KEY: &[u8] = b"refs";

// Owner
pub fn owner_store(storage: &mut dyn Storage) -> Singleton<CanonicalAddr> {
    singleton(storage, OWNER_KEY)
}

pub fn read_owner_store(storage: &dyn Storage) -> ReadonlySingleton<CanonicalAddr> {
    singleton_read(storage, OWNER_KEY)
}

// Relayer
pub fn relayers_store(storage: &mut dyn Storage) -> PrefixedStorage {
    PrefixedStorage::new(storage, RELAYERS_KEY)
}

pub fn read_relayers_store(storage: &dyn Storage) -> ReadonlyPrefixedStorage {
    ReadonlyPrefixedStorage::new(storage, RELAYERS_KEY)
}

// RefData
pub fn ref_data_store(storage: &mut dyn Storage) -> PrefixedStorage {
    PrefixedStorage::new(storage, REFS_KEY)
}

pub fn read_ref_data_store(storage: &dyn Storage) -> ReadonlyPrefixedStorage {
    ReadonlyPrefixedStorage::new(storage, REFS_KEY)
}
