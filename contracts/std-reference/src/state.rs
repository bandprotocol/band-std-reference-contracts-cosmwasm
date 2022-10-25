use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint256};
use cw_controllers::Admin;
use cw_storage_plus::Map;

// Administrator account
pub const ADMIN: Admin = Admin::new("admin");

// Used to store addresses of relayers and their state
pub const RELAYERS: Map<&Addr, bool> = Map::new("relayers");

// Used to store RefData
pub const REFDATA: Map<&str, RefData> = Map::new("refdata");

#[cw_serde]
pub struct RefData {
    // Rate of an asset relative to USD
    pub rate: Uint128,
    // The resolve time of the request ID
    pub resolve_time: u64,
    // The request ID where the rate was derived from
    pub request_id: u64,
}

impl RefData {
    pub fn new(rate: Uint128, resolve_time: u64, request_id: u64) -> Self {
        RefData {
            rate,
            resolve_time,
            request_id,
        }
    }
}

#[cw_serde]
pub struct ReferenceData {
    // Pair rate e.g. rate of BTC/USD
    pub rate: Uint256,
    // Unix time of when the base asset was last updated. e.g. Last update time of BTC in Unix time
    pub last_updated_base: u64,
    // Unix time of when the quote asset was last updated. e.g. Last update time of USD in Unix time
    pub last_updated_quote: u64,
}

impl ReferenceData {
    pub fn new(rate: Uint256, last_updated_base: u64, last_updated_quote: u64) -> Self {
        ReferenceData {
            rate,
            last_updated_base,
            last_updated_quote,
        }
    }
}
