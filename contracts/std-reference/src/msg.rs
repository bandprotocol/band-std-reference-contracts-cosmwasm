use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint64;

use crate::state::{RefData, ReferenceData};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    // Updates the contract config
    UpdateAdmin {
        // Address of the new owner
        admin: String,
    },
    // Whitelists addresses into relayer set
    AddRelayers {
        // Addresses of the to-be relayers
        relayers: Vec<String>,
    },
    // Removes addresses from relayer set
    RemoveRelayers {
        // Addresses to revoke the relayer rights
        relayers: Vec<String>,
    },
    // Relays a vector of symbols and their corresponding rates
    Relay {
        // A vector of symbols and their corresponding rates where:
        // symbol_rate := (symbol, rate)
        // e.g.
        // BTC = 19,343.34, ETH = 1,348.57
        // symbol_rates ≡ <("BTC", 19343340000000), ("ETH", 1348570000000)>
        symbol_rates: Vec<(String, Uint64)>,
        // Resolve time of request on BandChain in Unix timestamp
        resolve_time: Uint64,
        // Request ID of the results on BandChain
        request_id: Uint64,
    },
    // Same as Relay but without the resolve_time guard
    ForceRelay {
        symbol_rates: Vec<(String, Uint64)>,
        resolve_time: Uint64,
        request_id: Uint64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Returns admin accounts
    #[returns(cw_controllers::AdminResponse)]
    Admin {},
    // Queries if given a address is a relayer
    #[returns(bool)]
    IsRelayer {
        // Address to check relayer status
        relayer: String,
    },
    #[returns(RefData)]
    // Returns the RefData of a given symbol
    GetRef {
        // Symbol to query
        symbol: String,
    },
    #[returns(ReferenceData)]
    // Returns the ReferenceData of a given asset pairing
    GetReferenceData {
        // Symbol pair to query where:
        // symbol_pair := (base_symbol, quote_symbol)
        // e.g. BTC/USD ≡ ("BTC", "USD")
        symbol_pair: (String, String),
    },
    #[returns(Vec < ReferenceData >)]
    // Returns the ReferenceDatas of the given asset pairings
    GetReferenceDataBulk {
        // Vector of Symbol pair to query
        // e.g. <BTC/USD ETH/USD, BAND/BTC> ≡ <("BTC", "USD"), ("ETH", "USD"), ("BAND", "BTC")>
        symbol_pairs: Vec<(String, String)>,
    },
}
