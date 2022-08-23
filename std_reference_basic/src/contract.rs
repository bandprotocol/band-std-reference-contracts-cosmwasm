use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{CONFIG, REFDATA, RELAYERS};
use crate::struct_types::{Config, RefData, ReferenceData, Relayer};

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

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Addr,
) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(StdError::generic_err("NOT_AUTHORIZED"));
    }

    config.owner = new_owner;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn execute_add_relayers(
    deps: DepsMut,
    info: MessageInfo,
    relayers: Vec<Addr>,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(StdError::generic_err("NOT_AUTHORIZED"));
    }

    for relayer_addr in relayers {
        let relayer = Relayer {
            address: relayer_addr.clone(),
        };
        RELAYERS.save(deps.storage, &relayer_addr.to_string(), &relayer)?;
    }

    Ok(Response::new().add_attribute("action", "add_relayers"))
}

pub fn execute_remove_relayers(
    deps: DepsMut,
    info: MessageInfo,
    relayers: Vec<Addr>,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(StdError::generic_err("NOT_AUTHORIZED"));
    }

    for relayer_addr in relayers {
        RELAYERS.remove(deps.storage, &relayer_addr.to_string());
    }

    Ok(Response::new().add_attribute("action", "remove_relayers"))
}

pub fn execute_relay(
    deps: DepsMut,
    info: MessageInfo,
    symbols: Vec<String>,
    rates: Vec<Uint128>,
    resolve_time: u64,
    request_id: u64,
) -> StdResult<Response> {
    if !query_is_relayer(deps.as_ref(), info.sender).unwrap() {
        return Err(StdError::generic_err("NOT_A_RELAYER"));
    }

    if !(rates.len() == symbols.len()) {
        return Err(StdError::generic_err("NOT_ALL_INPUT_SIZES_ARE_THE_SAME"));
    }

    for (symbol, rate) in symbols.into_iter().zip(rates.into_iter()) {
        match REFDATA.may_load(deps.storage, &symbol)? {
            Some(existing_refdata) => {
                if existing_refdata.resolve_time < resolve_time {
                    REFDATA.save(
                        deps.storage,
                        &symbol,
                        &RefData::new(rate, resolve_time, request_id),
                    )?;
                } else {
                    return Err(StdError::generic_err("INVALID_RESOLVE_TIME"));
                }
            }
            None => REFDATA.save(
                deps.storage,
                &symbol,
                &RefData::new(rate, resolve_time, request_id),
            )?,
        }
    }

    Ok(Response::default().add_attribute("action", "execute_relay"))
}

pub fn execute_force_relay(
    deps: DepsMut,
    info: MessageInfo,
    symbols: Vec<String>,
    rates: Vec<Uint128>,
    resolve_time: u64,
    request_id: u64,
) -> StdResult<Response> {
    if !query_is_relayer(deps.as_ref(), info.sender).unwrap() {
        return Err(StdError::generic_err("NOT_A_RELAYER"));
    }

    if !(rates.len() == symbols.len()) {
        return Err(StdError::generic_err("NOT_ALL_INPUT_SIZES_ARE_THE_SAME"));
    }

    for (symbol, rate) in symbols.into_iter().zip(rates.into_iter()) {
        REFDATA.save(
            deps.storage,
            &symbol,
            &RefData::new(rate, resolve_time, request_id),
        )?;
    }

    Ok(Response::default().add_attribute("action", "execute_force_relay"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::IsRelayer { relayer } => to_binary(&query_is_relayer(deps, relayer)?),
        QueryMsg::GetRef { symbol } => to_binary(&query_ref(deps, symbol)?),
        QueryMsg::GetReferenceData {
            base_symbol,
            quote_symbol,
        } => to_binary(&query_reference_data(deps, base_symbol, quote_symbol)?),
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

fn query_config(deps: Deps) -> StdResult<Addr> {
    match CONFIG.may_load(deps.storage)? {
        Some(config) => Ok(config.owner),
        None => Err(StdError::generic_err("CONFIG_NOT_INITIALIZED")),
    }
}

fn query_is_relayer(deps: Deps, relayer: Addr) -> StdResult<bool> {
    match RELAYERS.may_load(deps.storage, &relayer.to_string())? {
        Some(_relayer) => Ok(true),
        None => Ok(false),
    }
}

fn query_ref(deps: Deps, symbol: String) -> StdResult<RefData> {
    if symbol == String::from("USD") {
        return Ok(RefData::new(Uint128::new(E9), u64::MAX, 0));
    }

    match REFDATA.may_load(deps.storage, &symbol)? {
        Some(refdata) => Ok(refdata),
        None => Err(StdError::generic_err(format!(
            "REF_DATA_NOT_AVAILABLE_FOR_{}",
            symbol
        ))),
    }
}

fn query_reference_data(
    deps: Deps,
    base_symbol: String,
    quote_symbol: String,
) -> StdResult<ReferenceData> {
    let base: RefData = query_ref(deps, base_symbol)?;
    let quote: RefData = query_ref(deps, quote_symbol)?;
    let base_rate: u128 = base.rate.into();
    let quote_rate: u128 = quote.rate.into();

    Ok(ReferenceData::new(
        Uint128::new((base_rate * E9 * E9) / quote_rate),
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
        .map(|(b, q)| query_reference_data(deps, b.clone(), q.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coins, from_binary, Coin, OwnedDeps, StdError, Timestamp};

    use super::*;

    fn init_msg() -> InstantiateMsg {
        InstantiateMsg {}
    }

    fn execute_transfer_ownership(o: &str) -> ExecuteMsg {
        ExecuteMsg::UpdateConfig {
            new_owner: Addr::unchecked(String::from(o)),
        }
    }

    fn execute_add_relayer(rs: Vec<&str>) -> ExecuteMsg {
        ExecuteMsg::AddRelayers {
            relayers: rs
                .into_iter()
                .map(|r| Addr::unchecked(String::from(r)))
                .collect(),
        }
    }

    fn execute_remove_relayer(rs: Vec<&str>) -> ExecuteMsg {
        ExecuteMsg::RemoveRelayers {
            relayers: rs
                .into_iter()
                .map(|r| Addr::unchecked(String::from(r)))
                .collect(),
        }
    }

    fn execute_relay(
        symbols: Vec<String>,
        rates: Vec<Uint128>,
        resolve_time: u64,
        request_id: u64,
    ) -> ExecuteMsg {
        ExecuteMsg::Relay {
            symbols,
            rates,
            resolve_time,
            request_id,
        }
    }

    fn execute_force_relay(
        symbols: Vec<String>,
        rates: Vec<Uint128>,
        resolve_time: u64,
        request_id: u64,
    ) -> ExecuteMsg {
        ExecuteMsg::ForceRelay {
            symbols,
            rates,
            resolve_time,
            request_id,
        }
    }

    fn query_config_msg() -> QueryMsg {
        QueryMsg::Config {}
    }

    fn query_is_relayer_msg(r: &str) -> QueryMsg {
        QueryMsg::IsRelayer {
            relayer: Addr::unchecked(String::from(r)),
        }
    }

    fn query_ref_data_msg(symbol: String) -> QueryMsg {
        QueryMsg::GetRef { symbol }
    }

    fn query_reference_data_msg(base_symbol: String, quote_symbol: String) -> QueryMsg {
        QueryMsg::GetReferenceData {
            base_symbol,
            quote_symbol,
        }
    }

    fn query_reference_data_bulk_msg(
        base_symbols: Vec<String>,
        quote_symbols: Vec<String>,
    ) -> QueryMsg {
        QueryMsg::GetReferenceDataBulk {
            base_symbols,
            quote_symbols,
        }
    }

    fn get_mocks(
        sender: &str,
        sent: &[Coin],
        height: u64,
        time: u64,
    ) -> (
        OwnedDeps<MockStorage, MockApi, MockQuerier>,
        Env,
        MessageInfo,
    ) {
        let deps = mock_dependencies();

        let mut env = mock_env();
        env.block.height = height;
        env.block.time = Timestamp::from_seconds(time);

        let info = mock_info(sender, sent);

        (deps, env, info)
    }

    #[test]
    fn proper_initialization() {
        let msg = init_msg();
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 789, 0);

        // owner not initialized yet
        match query(deps.as_ref(), env.clone(), query_config_msg()).unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!("CONFIG_NOT_INITIALIZED", msg),
            _ => panic!("Test Fail: expect CONFIG_NOT_INITIALIZED"),
        }

        // Check if successfully set owner
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // should be able to query
        let encoded_owner = query(deps.as_ref(), env.clone(), query_config_msg()).unwrap();
        // Verify correct owner address
        assert_eq!(
            String::from("owner"),
            from_binary::<Addr>(&encoded_owner).unwrap()
        );
    }

    #[test]
    fn test_transfer_ownership_fail_unauthorized() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 789, 0);

        // should successfully instantiate owner
        assert_eq!(
            0,
            instantiate(deps.as_mut(), env.clone(), info, init_msg())
                .unwrap()
                .messages
                .len()
        );

        // check owner in the state
        assert_eq!(
            String::from("owner"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_config_msg()).unwrap())
                .unwrap()
        );

        let (_, alice_env, alice_info) = get_mocks("alice", &coins(1000, "test_coin"), 789, 0);

        // should fail because sender is alice not owner
        match execute(
            deps.as_mut(),
            alice_env,
            alice_info,
            execute_transfer_ownership("new_owner"),
        )
        .unwrap_err()
        {
            StdError::GenericErr { msg, .. } => assert_eq!("NOT_AUTHORIZED", msg),
            _ => panic!("Test Fail: expect NOT_AUTHORIZED"),
        }
    }

    #[test]
    fn test_transfer_ownership_success() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 789, 0);

        // should successfully instantiate owner
        assert_eq!(
            0,
            instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg())
                .unwrap()
                .messages
                .len()
        );

        // // check owner in the state
        assert_eq!(
            String::from("owner"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_config_msg()).unwrap())
                .unwrap()
        );

        // should successfully set new owner
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_transfer_ownership("new_owner"),
            )
            .unwrap()
            .messages
            .len()
        );

        // check owner in the state should be new_owner
        assert_eq!(
            String::from("new_owner"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_config_msg()).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn test_add_remove_relayer_fail_not_authorized() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 876, 9999);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg()).unwrap();
        assert_eq!(0, res.messages.len());

        let (_, alice_env, alice_info) = get_mocks("alice", &coins(1000, "test_coin"), 876, 0);

        // NOT_AUTHORIZED, sender should be owner not alice
        match execute(
            deps.as_mut(),
            alice_env,
            alice_info,
            execute_add_relayer(vec!["r1", "r2", "r3"]),
        )
        .unwrap_err()
        {
            StdError::GenericErr { msg, .. } => assert_eq!("NOT_AUTHORIZED", msg),
            _ => panic!("Test Fail: expect NOT_AUTHORIZED"),
        }
    }

    #[test]
    fn test_add_remove_relayer() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 876, 9999);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg()).unwrap();
        assert_eq!(0, res.messages.len());

        let test_relayers = vec!["r1", "r2", "r3"];

        // Check that all "r" is not a relayer yet
        for r in test_relayers.clone() {
            assert_eq!(
                false,
                from_binary::<bool>(
                    &query(deps.as_ref(), env.clone(), query_is_relayer_msg(r)).unwrap()
                )
                .unwrap()
            );
        }

        // should successfully add all "r"
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_add_relayer(test_relayers.clone()),
            )
            .unwrap()
            .messages
            .len()
        );

        // Check that all "r" is now a relayer
        for r in test_relayers.clone() {
            assert_eq!(
                true,
                from_binary::<bool>(
                    &query(deps.as_ref(), env.clone(), query_is_relayer_msg(r)).unwrap()
                )
                .unwrap()
            );
        }

        // should successfully remove all "r"
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_remove_relayer(test_relayers.clone()),
            )
            .unwrap()
            .messages
            .len()
        );

        // Check that all "r" is removed
        for r in test_relayers.clone() {
            assert_eq!(
                false,
                from_binary::<bool>(
                    &query(deps.as_ref(), env.clone(), query_is_relayer_msg(r)).unwrap()
                )
                .unwrap()
            );
        }
    }

    #[test]
    fn test_query_usd() {
        let (deps, env, _info) = get_mocks("owner", &coins(1000, "test_coin"), 876, 9999);

        assert_eq!(
            RefData::new(Uint128::from(1 * E9), u64::MAX, 0),
            from_binary::<RefData>(
                &query(deps.as_ref(), env.clone(), query_ref_data_msg("USD".into())).unwrap()
            )
            .unwrap()
        );
    }

    #[test]
    fn test_handle_relay_fail_not_a_relayer() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 876, 0);
        let msg = init_msg();

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let (_, alice_env, alice_info) = get_mocks("alice", &coins(1000, "test_coin"), 876, 0);

        // NOT_A_RELAYER, sender should be a relayer
        match execute(
            deps.as_mut(),
            alice_env,
            alice_info,
            execute_relay(
                vec!["A".into(), "B".into(), "C".into()],
                vec![
                    Uint128::from(1 * E9),
                    Uint128::from(2 * E9),
                    Uint128::from(3 * E9),
                ],
                100,
                1,
            ),
        )
        .unwrap_err()
        {
            StdError::GenericErr { msg, .. } => assert_eq!("NOT_A_RELAYER", msg),
            _ => panic!("Test Fail: expect NOT_A_RELAYER"),
        }
    }

    #[test]
    fn test_handle_relay_success() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 876, 0);
        let msg = init_msg();

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Check that the owner is not a relayer yet
        assert_eq!(
            false,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully add relayer
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_add_relayer(vec!["owner"]),
            )
            .unwrap()
            .messages
            .len()
        );

        // Check that the owner is now a relayer
        assert_eq!(
            true,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully relay
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_relay(
                    vec!["A".into(), "B".into(), "C".into()],
                    vec![
                        Uint128::from(1 * E9),
                        Uint128::from(2 * E9),
                        Uint128::from(3 * E9),
                    ],
                    100,
                    1,
                ),
            )
            .unwrap()
            .messages
            .len()
        );

        // check ref data is correct
        assert_eq!(
            RefData::new(Uint128::from(1 * E9), 100, 1),
            from_binary::<RefData>(
                &query(deps.as_ref(), env.clone(), query_ref_data_msg("A".into())).unwrap()
            )
            .unwrap()
        );

        // should successfully relay
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_relay(
                    vec!["A".into(), "B".into(), "C".into()],
                    vec![
                        Uint128::from(4 * E9),
                        Uint128::from(5 * E9),
                        Uint128::from(6 * E9),
                    ],
                    110,
                    1,
                ),
            )
            .unwrap()
            .messages
            .len()
        );

        // check ref data is correct
        assert_eq!(
            RefData::new(Uint128::from(4 * E9), 110, 1),
            from_binary::<RefData>(
                &query(deps.as_ref(), env.clone(), query_ref_data_msg("A".into())).unwrap()
            )
            .unwrap()
        );
    }

    #[test]
    fn test_handle_relay_less_resolve_time() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 876, 0);
        let msg = init_msg();

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Check that the owner is not a relayer yet
        assert_eq!(
            false,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully add relayer
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_add_relayer(vec!["owner"]),
            )
            .unwrap()
            .messages
            .len()
        );

        // Check that the owner is now a relayer
        assert_eq!(
            true,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully relay first time
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_relay(
                    vec!["A".into(), "B".into()],
                    vec![Uint128::from(1 * E9), Uint128::from(2 * E9)],
                    100,
                    1,
                ),
            )
            .unwrap()
            .messages
            .len()
        );

        // should not relay second time
        // assert_eq!(
        //     0,
        //     execute(
        //         deps.as_mut(),
        //         env.clone(),
        //         info.clone(),
        //         execute_relay(
        //             vec!["A".into(), "B".into(), "C".into()],
        //             vec![
        //                 Uint128::from(4 * E9),
        //                 Uint128::from(5 * E9),
        //                 Uint128::from(6 * E9),
        //             ],
        //             90,
        //             1,
        //         ),
        //     )
        //     .unwrap()
        //     .messages
        //     .len()
        // );
        match execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            execute_relay(
                vec!["A".into(), "B".into(), "C".into()],
                vec![
                    Uint128::from(4 * E9),
                    Uint128::from(5 * E9),
                    Uint128::from(6 * E9),
                ],
                90,
                1,
            ),
        )
        .unwrap_err()
        {
            StdError::GenericErr { msg, .. } => assert_eq!("INVALID_RESOLVE_TIME", msg),
            _ => panic!("Test Fail: expect INVALID_RESOLVE_TIME"),
        }

        // check ref data is correct. A should be data from the first time.
        assert_eq!(
            RefData::new(Uint128::from(1 * E9), 100, 1),
            from_binary::<RefData>(
                &query(deps.as_ref(), env.clone(), query_ref_data_msg("A".into())).unwrap()
            )
            .unwrap()
        );

        // check ref data is correct. B should be data from the first time.
        assert_eq!(
            RefData::new(Uint128::from(2 * E9), 100, 1),
            from_binary::<RefData>(
                &query(deps.as_ref(), env.clone(), query_ref_data_msg("B".into())).unwrap()
            )
            .unwrap()
        );

        // check ref data is correct. C should not be available

        match query(deps.as_ref(), env.clone(), query_ref_data_msg("C".into())).unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!("REF_DATA_NOT_AVAILABLE_FOR_C", msg),
            _ => panic!("Test Fail: expect REF_DATA_NOT_AVAILABLE_FOR_C"),
        }

        // match query_ref_data_msg("C".into()).unwrap() {
        //     StdError::GenericErr { msg, .. } => assert_eq!("REF_DATA_NOT_AVAILABLE_FOR_C", msg),
        //     _ => panic!("Test Fail: expect REF_DATA_NOT_AVAILABLE_FOR_C"),
        // }
    }

    #[test]
    fn test_handle_force_relay_success() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 876, 0);
        let msg = init_msg();

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Check that the owner is not a relayer yet
        assert_eq!(
            false,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully add relayer
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_add_relayer(vec!["owner"]),
            )
            .unwrap()
            .messages
            .len()
        );

        // Check that the owner is now a relayer
        assert_eq!(
            true,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully relay
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_relay(
                    vec!["A".into(), "B".into()],
                    vec![Uint128::from(1 * E9), Uint128::from(2 * E9)],
                    100,
                    1,
                ),
            )
            .unwrap()
            .messages
            .len()
        );

        // should successfully force relay
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_force_relay(
                    vec!["A".into(), "B".into(), "C".into()],
                    vec![
                        Uint128::from(4 * E9),
                        Uint128::from(5 * E9),
                        Uint128::from(6 * E9),
                    ],
                    90,
                    1,
                ),
            )
            .unwrap()
            .messages
            .len()
        );

        // check ref data is correct. A should be data from force relay.
        assert_eq!(
            RefData::new(Uint128::from(4 * E9), 90, 1),
            from_binary::<RefData>(
                &query(deps.as_ref(), env.clone(), query_ref_data_msg("A".into())).unwrap()
            )
            .unwrap()
        );

        // check ref data is correct. B should be data from force relay.
        assert_eq!(
            RefData::new(Uint128::from(5 * E9), 90, 1),
            from_binary::<RefData>(
                &query(deps.as_ref(), env.clone(), query_ref_data_msg("B".into())).unwrap()
            )
            .unwrap()
        );

        // check ref data is correct. C should be data from force relay.
        assert_eq!(
            RefData::new(Uint128::from(6 * E9), 90, 1),
            from_binary::<RefData>(
                &query(deps.as_ref(), env.clone(), query_ref_data_msg("C".into())).unwrap()
            )
            .unwrap()
        );
    }

    #[test]
    fn test_handle_relay_and_query_reference_data() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 876, 1234);
        let msg = init_msg();

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Check that the owner is not a relayer yet
        assert_eq!(
            false,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully add relayer
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_add_relayer(vec!["owner"]),
            )
            .unwrap()
            .messages
            .len()
        );

        // Check that the owner is now a relayer
        assert_eq!(
            true,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully relay
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_relay(
                    vec!["A".into(), "B".into(), "C".into()],
                    vec![
                        Uint128::from(1 * E9),
                        Uint128::from(2 * E9),
                        Uint128::from(3 * E9),
                    ],
                    100,
                    1,
                ),
            )
            .unwrap()
            .messages
            .len()
        );

        // check 1
        assert_eq!(
            ReferenceData::new(Uint128::from(E9 * E9 / 2), 100, 100),
            from_binary::<ReferenceData>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    query_reference_data_msg("A".into(), "B".into()),
                )
                .unwrap()
            )
            .unwrap()
        );

        // check 2
        assert_eq!(
            ReferenceData::new(Uint128::from(E9 * E9 / 3), 100, 100),
            from_binary::<ReferenceData>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    query_reference_data_msg("A".into(), "C".into()),
                )
                .unwrap()
            )
            .unwrap()
        );

        // check 3
        assert_eq!(
            ReferenceData::new(Uint128::from(1 * E9 * E9), 100, u64::MAX),
            from_binary::<ReferenceData>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    query_reference_data_msg("A".into(), "USD".into()),
                )
                .unwrap()
            )
            .unwrap()
        );
    }

    #[test]
    fn test_handle_relay_and_query_reference_data_bulk() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 876, 1234);
        let msg = init_msg();

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Check that the owner is not a relayer yet
        assert_eq!(
            false,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully add relayer
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_add_relayer(vec!["owner"]),
            )
            .unwrap()
            .messages
            .len()
        );

        // Check that the owner is now a relayer
        assert_eq!(
            true,
            from_binary::<bool>(
                &query(deps.as_ref(), env.clone(), query_is_relayer_msg("owner")).unwrap()
            )
            .unwrap()
        );

        // should successfully relay
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_relay(
                    vec!["A".into(), "B".into(), "C".into()],
                    vec![
                        Uint128::from(1 * E9),
                        Uint128::from(2 * E9),
                        Uint128::from(3 * E9),
                    ],
                    100,
                    1,
                ),
            )
            .unwrap()
            .messages
            .len()
        );

        // check 1
        assert_eq!(
            vec![
                ReferenceData::new(Uint128::from(E9 * E9 / 3), 100, 100),
                ReferenceData::new(Uint128::from(2 * E9 * E9), 100, u64::MAX),
                ReferenceData::new(Uint128::from(3 * E9 * E9 / 2), 100, 100),
            ],
            from_binary::<Vec<ReferenceData>>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    query_reference_data_bulk_msg(
                        vec!["A".into(), "B".into(), "C".into()],
                        vec!["C".into(), "USD".into(), "B".into()],
                    ),
                )
                .unwrap()
            )
            .unwrap()
        );
    }
}
