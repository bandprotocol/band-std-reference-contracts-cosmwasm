use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Relayer {
    pub address: Addr,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, JsonSchema)]
pub struct ReferenceData {
    pub rate: Uint128,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}

impl ReferenceData {
    pub fn new(rate: Uint128, last_updated_base: u64, last_updated_quote: u64) -> Self {
        ReferenceData {
            rate,
            last_updated_base,
            last_updated_quote,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, JsonSchema)]
pub struct RefData {
    pub rate: Uint128,
    pub resolve_time: u64,
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn has_coins_matches() {
        let ref_data = RefData::new(Uint128::from(100u64), 200, 300);

        assert_eq!(Uint128::from(100u64), ref_data.rate);
        assert_eq!(200, ref_data.resolve_time);
        assert_eq!(300, ref_data.request_id);
    }
}
