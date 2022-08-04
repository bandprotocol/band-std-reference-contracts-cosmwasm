# Band Protocol's Cosmwasm Standard Reference Contracts

## Overview

This repository contains the CosmWasm code for Band Protocol's StdReference contracts. The live contract
addresses can be found in
our [documentation](https://docs.bandchain.org/band-standard-dataset/supported-blockchains.html).

## Usage

To query the prices from Band Protocol's StdReference contracts, the contract looking to use the price values should
query Band Protocol's `std_reference_proxy` contract.

### QueryMsg

Acceptable query messages for the `std_reference_proxy` contract are as follows:

```rust
pub enum QueryMsg {
    GetReferenceData {
        base_symbol: String,
        quote_symbol: String,
    },
    GetReferenceDataBulk {
        base_symbols: Vec<String>,
        quote_symbols: Vec<String>,
    },
}
```

### ReferenceData

The `ReferenceData` struct is defined as:

```rust
pub struct ReferenceData {
    pub rate: Uint128,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}
```

where the struct variables:

- `rate` is defined as the base/quote exchange rate multiplied by 1e18.
- `lastUpdatedBase` is defined as the UNIX epoch of the last time the base price was updated.
- `lastUpdatedQuote` is defined as the UNIX epoch of the last time the quote price was updated.

### GetReferenceData

#### Input

- The base symbol as type `String`
- The quote symbol as type `String`

#### Output

- The base quote pair result as type `ReferenceData`

#### Example

For example, if we wanted to query the price of `BTC/USD`, the demo function below shows how this can be done.

```rust
fn demo(
    proxy_address: Addr,
    base_symbol: String,
    quote_symbol: String,
) -> StdResult<ReferenceData> {
    deps.querier.query_wasm_smart(
        &proxy_address,
        &QueryMsg::GetReferenceData {
            base_symbol,
            quote_symbol,
        },
    )
}
```

Where the result from `demo(proxy_address, "BTC", "USD")` would yield:

```
ReferenceData(23131270000000000000000, 1659588229, 1659589497)
```

and the results can be interpreted as:

- BTC/USD
    - `rate = 23131.27 BTC/USD`
    - `lastUpdatedBase = 1659588229`
    - `lastUpdatedQuote = 1659589497`

### GetReferenceDataBulk

#### Input

- A vector of base symbols as type `Vec<String>`
- A vector of quote symbol as type `Vec<String>`

#### Output

- A vector of the base quote pair results as type `Vec<ReferenceData>`

#### Example

For example, if we wanted to query the price of `BTC/USD` and `ETH/BTC`, the demo contract below shows how this can be
done.

```rust
fn demo(
    proxy_address: Addr,
    base_symbols: Vec<String>,
    quote_symbols: Vec<String>,
) -> StdResult<ReferenceData> {
    deps.querier.query_wasm_smart(
        &proxy_address,
        &QueryMsg::GetReferenceDataBulk {
            base_symbols,
            quote_symbols,
        },
    )
}
```

Where the result from `demo(proxy_address, ["BTC", "ETH"], ["USD", "BTC"])` would yield:

```
[
    ReferenceData(23131270000000000000000, 1659588229, 1659589497),
    ReferenceData(71601775432131482, 1659588229, 1659588229)
]
```

and the results can be interpreted as:

- BTC/USD
    - `rate = 23131.27 BTC/USD`
    - `lastUpdatedBase = 1659588229`
    - `lastUpdatedQuote = 1659589497`
- ETH/BTC
    - `rate = 0.07160177543213148 ETH/BTC`
    - `lastUpdatedBase = 1659588229`
    - `lastUpdatedQuote = 1659588229`
