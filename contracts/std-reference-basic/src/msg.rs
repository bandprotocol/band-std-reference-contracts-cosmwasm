use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        new_owner: Addr,
    },
    AddRelayers {
        relayers: Vec<Addr>,
    },
    RemoveRelayers {
        relayers: Vec<Addr>,
    },
    Relay {
        symbols: Vec<String>,
        rates: Vec<Uint128>,
        resolve_time: u64,
        request_id: u64,
    },
    ForceRelay {
        symbols: Vec<String>,
        rates: Vec<Uint128>,
        resolve_time: u64,
        request_id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    IsRelayer {
        relayer: Addr,
    },
    GetRef {
        symbol: String,
    },
    GetReferenceData {
        base_symbol: String,
        quote_symbol: String,
    },
    GetReferenceDataBulk {
        base_symbols: Vec<String>,
        quote_symbols: Vec<String>,
    },
}
