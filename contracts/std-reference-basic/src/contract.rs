use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{CONFIG, REFDATA, RELAYERS};
use crate::struct_types::{Config, RefData, ReferenceData};

pub static E9: u128 = 1_000_000_000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    CONFIG.save(deps.storage, &Config { owner: info.sender })?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { new_owner } => execute_update_config(deps, info, new_owner),
        ExecuteMsg::AddRelayers { relayers } => execute_add_relayers(deps, info, relayers),
        ExecuteMsg::RemoveRelayers { relayers } => execute_remove_relayers(deps, info, relayers),
        ExecuteMsg::Relay {
            symbols,
            rates,
            resolve_time,
            request_id,
        } => execute_relay(deps, info, symbols, rates, resolve_time, request_id),
        ExecuteMsg::ForceRelay {
            symbols,
            rates,
            resolve_time,
            request_id,
        } => execute_force_relay(deps, info, symbols, rates, resolve_time, request_id),
    }
}

fn is_owner(storage: &mut dyn Storage, sender: &Addr) -> StdResult<()> {
    let config = CONFIG.load(storage)?;
    if *sender != config.owner {
        Err(StdError::generic_err("NOT_AUTHORIZED"))
    } else {
        Ok(())
    }
}

fn execute_update_config(deps: DepsMut, info: MessageInfo, new_owner: Addr) -> StdResult<Response> {
    is_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    config.owner = new_owner;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_add_relayers(
    deps: DepsMut,
    info: MessageInfo,
    relayers: Vec<Addr>,
) -> StdResult<Response> {
    is_owner(deps.storage, &info.sender)?;

    for relayer in relayers {
        RELAYERS.save(deps.storage, &relayer, &true)?;
    }

    Ok(Response::new().add_attribute("action", "add_relayers"))
}

fn execute_remove_relayers(
    deps: DepsMut,
    info: MessageInfo,
    relayers: Vec<Addr>,
) -> StdResult<Response> {
    is_owner(deps.storage, &info.sender)?;

    for relayer in relayers {
        RELAYERS.remove(deps.storage, &relayer);
    }

    Ok(Response::new().add_attribute("action", "remove_relayers"))
}

fn execute_relay(
    deps: DepsMut,
    info: MessageInfo,
    symbols: Vec<String>,
    rates: Vec<Uint128>,
    resolve_time: u64,
    request_id: u64,
) -> StdResult<Response> {
    if !query_is_relayer(deps.as_ref(), &info.sender)? {
        return Err(StdError::generic_err("NOT_A_RELAYER"));
    }

    if symbols.len() != rates.len() {
        return Err(StdError::generic_err("MISMATCHED_INPUT_SIZES"));
    }

    for (symbol, rate) in symbols.iter().zip(rates.into_iter()) {
        if let Some(existing_refdata) = REFDATA.may_load(deps.storage, symbol)? {
            if existing_refdata.resolve_time >= resolve_time {
                continue;
            }
        }
        REFDATA.save(
            deps.storage,
            symbol,
            &RefData::new(rate, resolve_time, request_id),
        )?
    }

    Ok(Response::default().add_attribute("action", "execute_relay"))
}

fn execute_force_relay(
    deps: DepsMut,
    info: MessageInfo,
    symbols: Vec<String>,
    rates: Vec<Uint128>,
    resolve_time: u64,
    request_id: u64,
) -> StdResult<Response> {
    if !query_is_relayer(deps.as_ref(), &info.sender)? {
        return Err(StdError::generic_err("NOT_A_RELAYER"));
    }

    if symbols.len() != rates.len() {
        return Err(StdError::generic_err("NOT_ALL_INPUT_SIZES_ARE_THE_SAME"));
    }

    for (symbol, rate) in symbols.iter().zip(rates.into_iter()) {
        REFDATA.save(
            deps.storage,
            symbol,
            &RefData::new(rate, resolve_time, request_id),
        )?;
    }

    Ok(Response::default().add_attribute("action", "execute_force_relay"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::IsRelayer { relayer } => to_binary(&query_is_relayer(deps, &relayer)?),
        QueryMsg::GetRef { symbol } => to_binary(&query_ref(deps, &symbol)?),
        QueryMsg::GetReferenceData {
            base_symbol,
            quote_symbol,
        } => to_binary(&query_reference_data(deps, &base_symbol, &quote_symbol)?),
        QueryMsg::GetReferenceDataBulk {
            base_symbols,
            quote_symbols,
        } => to_binary(&query_reference_data_bulk(
            deps,
            base_symbols,
            quote_symbols,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_is_relayer(deps: Deps, relayer: &Addr) -> StdResult<bool> {
    Ok(RELAYERS.may_load(deps.storage, relayer)?.is_some())
}

fn query_ref(deps: Deps, symbol: &str) -> StdResult<RefData> {
    if symbol == "USD" {
        return Ok(RefData::new(Uint128::new(E9), u64::MAX, 0));
    }

    REFDATA.load(deps.storage, symbol)
}

fn query_reference_data(
    deps: Deps,
    base_symbol: &str,
    quote_symbol: &str,
) -> StdResult<ReferenceData> {
    let base = query_ref(deps, base_symbol)?;
    let quote = query_ref(deps, quote_symbol)?;

    Ok(ReferenceData::new(
        base.rate * Uint128::new(E9 * E9) / quote.rate,
        base.resolve_time,
        quote.resolve_time,
    ))
}

fn query_reference_data_bulk(
    deps: Deps,
    base_symbols: Vec<String>,
    quote_symbols: Vec<String>,
) -> StdResult<Vec<ReferenceData>> {
    if base_symbols.len() != quote_symbols.len() {
        return Err(StdError::generic_err("NOT_ALL_INPUT_SIZES_ARE_THE_SAME"));
    }

    base_symbols
        .iter()
        .zip(quote_symbols.iter())
        .map(|(b, q)| query_reference_data(deps, b, q))
        .collect()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Addr;

    use crate::msg::ExecuteMsg::{AddRelayers, Relay};

    use super::*;

    // This function will setup the contract for other tests
    fn setup(mut deps: DepsMut, sender: &str) {
        let info = mock_info(sender, &[]);
        let env = mock_env();
        let res = instantiate(deps.branch(), env, info, InstantiateMsg {}).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(
            query_config(deps.as_ref()).unwrap(),
            Config {
                owner: Addr::unchecked(sender)
            }
        )
    }

    fn is_relayers(deps: Deps, relayers: &[Addr]) -> Vec<bool> {
        relayers
            .iter()
            .map(|r| query_is_relayer(deps, r))
            .collect::<StdResult<Vec<bool>>>()
            .unwrap()
    }

    // This function will setup the relayer for other tests
    fn setup_relayers(mut deps: DepsMut, sender: &str, relayers: Vec<Addr>) {
        setup(deps.branch(), sender);

        let info = mock_info(sender, &[]);
        let env = mock_env();
        let msg = AddRelayers {
            relayers: relayers
                .clone()
                .into_iter()
                .map(|r| Addr::unchecked(r))
                .collect::<Vec<Addr>>(),
        };
        execute(deps.branch(), env, info, msg).unwrap();

        // Check if relayer is successfully added
        assert_eq!(
            is_relayers(deps.as_ref(), &relayers),
            std::iter::repeat(true)
                .take(relayers.len())
                .collect::<Vec<bool>>()
        );
    }

    // This function will setup mock relays for other tests
    fn setup_relays(
        mut deps: DepsMut,
        sender: &str,
        relayers: Vec<Addr>,
        symbols: Vec<String>,
        rates: Vec<Uint128>,
        resolve_time: u64,
        request_id: u64,
    ) {
        setup_relayers(deps.branch(), sender, relayers.clone());

        let info = mock_info(relayers[0].as_str(), &[]);
        let env = mock_env();

        let msg = Relay {
            symbols: symbols.clone(),
            rates: rates.clone(),
            resolve_time,
            request_id,
        };
        execute(deps.branch(), env, info, msg).unwrap();

        let reference_datas = query_reference_data_bulk(
            deps.as_ref(),
            symbols.clone(),
            std::iter::repeat("USD".to_string())
                .take(symbols.len())
                .collect::<Vec<String>>(),
        )
        .unwrap();

        let retrieved_rates = reference_datas
            .into_iter()
            .map(|rd| rd.rate / Uint128::new(E9))
            .collect::<Vec<Uint128>>();

        assert_eq!(retrieved_rates, rates);
    }

    mod instantiate {
        use super::*;

        #[test]
        fn can_instantiate() {
            let mut deps = mock_dependencies();
            let init_msg = InstantiateMsg {};
            let info = mock_info("owner", &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                query_config(deps.as_ref()).unwrap(),
                Config {
                    owner: Addr::unchecked("owner")
                }
            );
        }
    }

    mod config {
        use crate::msg::ExecuteMsg::UpdateConfig;

        use super::*;

        #[test]
        fn can_update_config_by_owner() {
            // Setup
            let mut deps = mock_dependencies();
            setup(deps.as_mut(), "owner");

            // Test authorized attempt to update config
            let info = mock_info("owner", &[]);
            let env = mock_env();
            let msg = UpdateConfig {
                new_owner: Addr::unchecked("new_owner"),
            };
            execute(deps.as_mut(), env, info, msg).unwrap();
            let config = query_config(deps.as_ref()).unwrap();
            assert_eq!(
                config,
                Config {
                    owner: Addr::unchecked("new_owner"),
                },
                "Expected successful owner change"
            );
        }

        #[test]
        fn cannot_update_config_by_others() {
            // Setup
            let mut deps = mock_dependencies();
            setup(deps.as_mut(), "owner");

            // Test unauthorized attempt to update config
            let info = mock_info("user", &[]);
            let env = mock_env();
            let msg = UpdateConfig {
                new_owner: Addr::unchecked("user"),
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, StdError::generic_err("NOT_AUTHORIZED"));
        }
    }

    mod relay {
        use crate::msg::ExecuteMsg::{AddRelayers, ForceRelay, Relay, RemoveRelayers};

        use super::*;

        #[test]
        fn add_relayers_by_owner() {
            // Setup
            let mut deps = mock_dependencies();
            let init_msg = InstantiateMsg {};
            let info = mock_info("owner", &[]);
            let env = mock_env();
            instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();
            let relayers_to_add: Vec<Addr> = vec!["relayer_1", "relayer_2", "relayer_3"]
                .into_iter()
                .map(Addr::unchecked)
                .collect();

            // Test authorized attempt to add relayers
            let info = mock_info("owner", &[]);
            let env = mock_env();
            let msg = AddRelayers {
                relayers: relayers_to_add.clone(),
            };
            execute(deps.as_mut(), env, info, msg).unwrap();

            assert_eq!(
                is_relayers(deps.as_ref(), &relayers_to_add),
                [true, true, true]
            );
        }

        #[test]
        fn add_relayers_by_other() {
            // Setup
            let mut deps = mock_dependencies();
            let init_msg = InstantiateMsg {};
            let info = mock_info("owner", &[]);
            let env = mock_env();
            instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

            // Test unauthorized attempt to add relayer
            let info = mock_info("user", &[]);
            let env = mock_env();
            let msg = AddRelayers {
                relayers: vec![Addr::unchecked("relayer_1")],
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, StdError::generic_err("NOT_AUTHORIZED"));
        }

        #[test]
        fn remove_relayers_by_owner() {
            // Setup
            let mut deps = mock_dependencies();
            let relayers_list = vec!["relayer_1", "relayer_2", "relayer_3"]
                .into_iter()
                .map(Addr::unchecked)
                .collect::<Vec<Addr>>();
            setup_relayers(deps.as_mut(), "owner", relayers_list.clone());

            // Remove relayer
            let relayers_to_remove = relayers_list[..2].to_vec();
            let info = mock_info("owner", &[]);
            let env = mock_env();
            let msg = RemoveRelayers {
                relayers: relayers_to_remove,
            };
            execute(deps.as_mut(), env, info, msg).unwrap();

            assert_eq!(
                is_relayers(deps.as_ref(), &relayers_list),
                [false, false, true]
            );
        }

        #[test]
        fn remove_relayers_by_other() {
            // Setup
            let mut deps = mock_dependencies();
            let relayers = vec![Addr::unchecked("relayer_1")];
            setup_relayers(deps.as_mut(), "owner", relayers.clone());

            // Test unauthorized attempt to remove relayer
            let info = mock_info("user", &[]);
            let env = mock_env();
            let msg = RemoveRelayers { relayers: relayers };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, StdError::generic_err("NOT_AUTHORIZED"));
        }

        #[test]
        fn attempt_relay_by_relayer() {
            // Setup
            let mut deps = mock_dependencies();
            let relayer = Addr::unchecked("relayer");
            setup_relayers(deps.as_mut(), "owner", vec![relayer.clone()]);

            // Test authorized attempt to relay data
            let info = mock_info(relayer.as_str(), &[]);
            let env = mock_env();
            let symbols = vec!["AAA", "BBB", "CCC"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            let rates = [1000, 2000, 3000]
                .iter()
                .map(|r| Uint128::new(*r))
                .collect::<Vec<Uint128>>();
            let msg = Relay {
                symbols: symbols.clone(),
                rates: rates.clone(),
                resolve_time: 100,
                request_id: 1,
            };
            execute(deps.as_mut(), env, info, msg).unwrap();

            // Check if relay was successful
            let reference_datas = query_reference_data_bulk(
                deps.as_ref(),
                symbols.clone(),
                std::iter::repeat("USD".to_string())
                    .take(symbols.len())
                    .collect::<Vec<String>>(),
            )
            .unwrap();
            let retrieved_rates = reference_datas
                .clone()
                .into_iter()
                .map(|rd| rd.rate / Uint128::new(E9))
                .collect::<Vec<Uint128>>();
            assert_eq!(retrieved_rates, rates.clone());
        }

        #[test]
        fn attempt_relay_by_relayer_with_mismatched_input_sizes() {
            // Setup
            let mut deps = mock_dependencies();
            let relayer = Addr::unchecked("relayer");
            setup_relayers(deps.as_mut(), "owner", vec![relayer.clone()]);

            // Test attempt to relay with mismatched input sizes
            let info = mock_info(relayer.as_str(), &[]);
            let env = mock_env();
            let symbols = vec!["AAA", "BBB", "CCC"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            let mismatched_rates = vec![Uint128::new(1)];
            let msg = Relay {
                symbols,
                rates: mismatched_rates,
                resolve_time: 100,
                request_id: 1,
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, StdError::generic_err("MISMATCHED_INPUT_SIZES"))
        }

        #[test]
        fn attempt_relay_by_relayer_with_invalid_resolve_time() {
            // Setup
            let mut deps = mock_dependencies();
            let relayer = Addr::unchecked("relayer");
            setup_relayers(deps.as_mut(), "owner", vec![relayer.clone()]);

            // Relay initial set of data
            let info = mock_info(relayer.as_str(), &[]);
            let env = mock_env();
            let symbols = vec!["AAA", "BBB", "CCC"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            let rates = [1000, 2000, 3000]
                .iter()
                .map(|r| Uint128::new(*r))
                .collect::<Vec<Uint128>>();
            let msg = Relay {
                symbols: symbols.clone(),
                rates: rates.clone(),
                resolve_time: 100,
                request_id: 1,
            };
            execute(deps.as_mut(), env, info, msg).unwrap();

            // Test attempt to relay with invalid resolve times
            let info = mock_info(relayer.as_str(), &[]);
            let env = mock_env();
            let old_rates = [1, 2, 3]
                .iter()
                .map(|r| Uint128::new(*r))
                .collect::<Vec<Uint128>>();
            let msg = Relay {
                symbols: symbols.clone(),
                rates: old_rates,
                resolve_time: 90,
                request_id: 1,
            };
            execute(deps.as_mut(), env, info, msg).unwrap();

            // Check if relay was successful
            let reference_datas = query_reference_data_bulk(
                deps.as_ref(),
                symbols.clone(),
                std::iter::repeat("USD".to_string())
                    .take(*&symbols.len())
                    .collect::<Vec<String>>(),
            )
            .unwrap();
            let retrieved_rates = reference_datas
                .clone()
                .into_iter()
                .map(|rd| rd.rate / Uint128::new(E9))
                .collect::<Vec<Uint128>>();
            assert_eq!(retrieved_rates, rates);
        }

        #[test]
        fn attempt_relay_by_others() {
            // Setup
            let mut deps = mock_dependencies();
            setup(deps.as_mut(), "owner");

            // Test unauthorized attempt to relay data
            let info = mock_info("user", &[]);
            let env = mock_env();
            let symbols = vec!["AAA", "BBB", "CCC"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            let rates = [1000, 2000, 3000]
                .iter()
                .map(|r| Uint128::new(*r))
                .collect::<Vec<Uint128>>();
            let msg = Relay {
                symbols: symbols.clone(),
                rates: rates.clone(),
                resolve_time: 0,
                request_id: 0,
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, StdError::generic_err("NOT_A_RELAYER"));
        }

        #[test]
        fn attempt_force_relay_by_relayer() {
            // Setup
            let mut deps = mock_dependencies();
            let relayer = Addr::unchecked("relayer");
            setup_relayers(deps.as_mut(), "owner", vec![relayer.clone()]);

            // Test authorized attempt to relay data
            let info = mock_info(relayer.as_str(), &[]);
            let env = mock_env();
            let symbols = vec!["AAA", "BBB", "CCC"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            let rates = [1000, 2000, 3000]
                .iter()
                .map(|r| Uint128::new(*r))
                .collect::<Vec<Uint128>>();
            let msg = ForceRelay {
                symbols: symbols.clone(),
                rates: rates.clone(),
                resolve_time: 100,
                request_id: 2,
            };
            execute(deps.as_mut(), env, info, msg).unwrap();

            // Test attempt to force relay
            let info = mock_info(relayer.as_str(), &[]);
            let env = mock_env();
            let forced_rates = [1, 2, 3]
                .iter()
                .map(|r| Uint128::new(*r))
                .collect::<Vec<Uint128>>();
            let msg = ForceRelay {
                symbols: symbols.clone(),
                rates: forced_rates.clone(),
                resolve_time: 90,
                request_id: 1,
            };
            execute(deps.as_mut(), env, info, msg).unwrap();

            // Check if forced relay was successful
            let reference_datas = query_reference_data_bulk(
                deps.as_ref(),
                symbols.clone(),
                std::iter::repeat("USD".to_string())
                    .take(symbols.len())
                    .collect::<Vec<String>>(),
            )
            .unwrap();
            let retrieved_rates = reference_datas
                .into_iter()
                .map(|rd| rd.rate / Uint128::new(E9))
                .collect::<Vec<Uint128>>();
            assert_eq!(retrieved_rates, forced_rates);
        }

        #[test]
        fn attempt_force_relay_by_other() {
            // Setup
            let mut deps = mock_dependencies();
            setup(deps.as_mut(), "owner");

            // Test unauthorized attempt to relay data
            let info = mock_info("user", &[]);
            let env = mock_env();
            let symbols = vec!["AAA", "BBB", "CCC"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            let rates = [1000, 2000, 3000]
                .iter()
                .map(|r| Uint128::new(*r))
                .collect::<Vec<Uint128>>();
            let msg = ForceRelay {
                symbols: symbols.clone(),
                rates: rates.clone(),
                resolve_time: 0,
                request_id: 0,
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, StdError::generic_err("NOT_A_RELAYER"));
        }
    }

    mod query {
        use cosmwasm_std::from_binary;

        use crate::msg::QueryMsg::{GetRef, GetReferenceData, GetReferenceDataBulk};

        use super::*;

        #[test]
        fn attempt_query_config() {
            // Setup
            let mut deps = mock_dependencies();
            setup(deps.as_mut(), "owner");

            // Test if query_config results are correct
            assert_eq!(
                query_config(deps.as_ref()).unwrap(),
                Config {
                    owner: Addr::unchecked("owner")
                }
            );
        }

        #[test]
        fn attempt_query_is_relayer() {
            let mut deps = mock_dependencies();
            let relayer = Addr::unchecked("relayer");
            setup_relayers(deps.as_mut(), "owner", vec![relayer.clone()]);

            // Test if is_relayer results are correct
            assert_eq!(query_is_relayer(deps.as_ref(), &relayer).unwrap(), true);
            assert_eq!(
                query_is_relayer(deps.as_ref(), &Addr::unchecked("not_a_relayer")).unwrap(),
                false
            );
        }

        #[test]
        fn attempt_query_get_ref() {
            // Setup
            let mut deps = mock_dependencies();
            let relayer = Addr::unchecked("relayer");
            let symbol = vec!["AAA".to_string()];
            let rate = vec![Uint128::new(1000)];
            setup_relays(
                deps.as_mut(),
                "owner",
                vec![relayer.clone()],
                symbol.clone(),
                rate.clone(),
                100,
                1,
            );

            // Test if get_ref results are correct
            let env = mock_env();
            let msg = GetRef {
                symbol: symbol[0].to_owned(),
            };
            let binary_res = query(deps.as_ref(), env, msg).unwrap();
            assert_eq!(
                from_binary::<RefData>(&binary_res).unwrap(),
                RefData::new(rate[0], 100, 1)
            );

            // Test invalid symbol
            let env = mock_env();
            let msg = GetRef {
                symbol: "DNE".to_string(),
            };
            let err = query(deps.as_ref(), env, msg).unwrap_err();
            assert_eq!(
                err,
                StdError::not_found("std_reference_basic::struct_types::RefData")
            );
        }

        #[test]
        fn attempt_query_get_reference_data() {
            // Setup
            let mut deps = mock_dependencies();
            let relayer = Addr::unchecked("relayer");
            let symbol = vec!["AAA".to_string()];
            let rate = vec![Uint128::new(1000)];
            setup_relays(
                deps.as_mut(),
                "owner",
                vec![relayer.clone()],
                symbol.clone(),
                rate.clone(),
                100,
                1,
            );

            // Test if get_reference_data results are correct
            let env = mock_env();
            let msg = GetReferenceData {
                base_symbol: symbol[0].to_owned(),
                quote_symbol: "USD".to_string(),
            };
            let binary_res = query(deps.as_ref(), env, msg).unwrap();
            assert_eq!(
                from_binary::<ReferenceData>(&binary_res).unwrap(),
                ReferenceData::new(rate[0] * Uint128::new(E9), 100, u64::MAX)
            );

            // Test invalid symbol
            let env = mock_env();
            let msg = GetReferenceData {
                base_symbol: "DNE".to_string(),
                quote_symbol: "USD".to_string(),
            };
            let err = query(deps.as_ref(), env, msg).unwrap_err();
            assert_eq!(
                err,
                StdError::not_found("std_reference_basic::struct_types::RefData")
            );
            // Test invalid symbols
            let env = mock_env();
            let msg = GetReferenceData {
                base_symbol: "DNE1".to_string(),
                quote_symbol: "DNE2".to_string(),
            };
            let err = query(deps.as_ref(), env, msg).unwrap_err();
            assert_eq!(
                err,
                StdError::not_found("std_reference_basic::struct_types::RefData")
            );
        }

        #[test]
        fn attempt_query_get_reference_data_bulk() {
            // Setup
            let mut deps = mock_dependencies();
            let relayer = Addr::unchecked("relayer");
            let symbols = vec!["AAA", "BBB", "CCC"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            let rates = [1000, 2000, 3000]
                .iter()
                .map(|r| Uint128::new(*r))
                .collect::<Vec<Uint128>>();
            setup_relays(
                deps.as_mut(),
                "owner",
                vec![relayer.clone()],
                symbols.clone(),
                rates.clone(),
                100,
                1,
            );

            // Test if get_reference_data results are correct
            let env = mock_env();
            let msg = GetReferenceDataBulk {
                base_symbols: symbols.clone(),
                quote_symbols: std::iter::repeat("USD")
                    .take(symbols.len())
                    .map(|q| q.to_string())
                    .collect::<Vec<String>>(),
            };
            let binary_res = query(deps.as_ref(), env, msg).unwrap();
            let expected_res = rates
                .iter()
                .map(|r| ReferenceData::new(r * Uint128::new(E9), 100, u64::MAX))
                .collect::<Vec<ReferenceData>>();
            assert_eq!(
                from_binary::<Vec<ReferenceData>>(&binary_res).unwrap(),
                expected_res
            );

            // Test invalid symbols
            let env = mock_env();
            let msg = GetReferenceDataBulk {
                base_symbols: vec!["AAA", "DNE1", "DNE2"]
                    .into_iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<String>>(),
                quote_symbols: std::iter::repeat("USD")
                    .take(3)
                    .map(|q| q.to_string())
                    .collect::<Vec<String>>(),
            };
            let err = query(deps.as_ref(), env, msg).unwrap_err();
            assert_eq!(
                err,
                StdError::not_found("std_reference_basic::struct_types::RefData")
            );

            // Test invalid symbols
            let env = mock_env();
            let msg = GetReferenceDataBulk {
                base_symbols: vec!["AAA", "DNE2", "BBB"]
                    .into_iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<String>>(),
                quote_symbols: vec!["DNE1", "DNE2", "DNE1"]
                    .into_iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<String>>(),
            };
            let err = query(deps.as_ref(), env, msg).unwrap_err();
            assert_eq!(
                err,
                StdError::not_found("std_reference_basic::struct_types::RefData")
            );
        }
    }
}
