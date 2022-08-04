use cosmwasm_std::{CanonicalAddr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

pub static OWNER_KEY: &[u8] = b"owner";
pub static REFS_KEY: &[u8] = b"ref";

pub fn owner_store(storage: &mut dyn Storage) -> Singleton<CanonicalAddr> {
    singleton(storage, OWNER_KEY)
}

pub fn read_owner_store(storage: &dyn Storage) -> ReadonlySingleton<CanonicalAddr> {
    singleton_read(storage, OWNER_KEY)
}

pub fn ref_contract_store(storage: &mut dyn Storage) -> Singleton<CanonicalAddr> {
    singleton(storage, REFS_KEY)
}

pub fn read_ref_contract_store(storage: &dyn Storage) -> ReadonlySingleton<CanonicalAddr> {
    singleton_read(storage, REFS_KEY)
}
