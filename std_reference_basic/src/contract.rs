use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    owner_store, read_owner_store, ref_data_store, read_ref_data_store, relayers_store,
    read_relayers_store
};
use crate::struct_types::{RefData, ReferenceData};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Storage, Uint128,
};

pub static E9: u128 = 1_000_000_000;

macro_rules! zip {
    ($x: expr) => ($x);
    ($x: expr, $($y: expr), +) => (
        $x.iter().map(|v| v.clone()).zip(zip!($($y.clone()), +))
    )
}

macro_rules! unwrap_or_return_err {
    ( $e:expr ) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Err(e),
        }
    };
}

macro_rules! unwrap_query {
    ( $e:expr, $f:expr ) => {
        match $e {
            Ok(x) => match to_binary(&x) {
                Ok(y) => Ok(y),
                Err(_) => Err(StdError::generic_err($f)),
            },
            Err(e) => return Err(e),
        }
    };
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    owner_store(deps.storage).save(&deps.api.addr_canonicalize(&info.sender.as_str())?)?;
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
        ExecuteMsg::TransferOwnership { new_owner } => {
            try_transfer_ownership(deps, info, new_owner)
        }
        ExecuteMsg::AddRelayers { relayers } => try_add_relayers(deps, info, relayers),
        ExecuteMsg::RemoveRelayers { relayers } => try_remove_relayers(deps, info, relayers),
        ExecuteMsg::Relay {
            symbols,
            rates,
            resolve_time,
            request_id,
        } => try_relay(deps, info, symbols, rates, resolve_time, request_id),
        ExecuteMsg::ForceRelay {
            symbols,
            rates,
            resolve_time,
            request_id,
        } => try_force_relay(deps, info, symbols, rates, resolve_time, request_id),
    }
}

pub fn try_transfer_ownership(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Addr,
) -> StdResult<Response> {
    let owner_addr = read_owner_store(deps.storage).load()?;
    if deps.api.addr_canonicalize(&info.sender.as_str())? != owner_addr {
        return Err(StdError::generic_err("NOT_AUTHORIZED"));
    }

    owner_store(deps.storage).save(&deps.api.addr_canonicalize(&new_owner.as_str())?)?;

    Ok(Response::default())
}

pub fn try_add_relayers(
    deps: DepsMut,
    info: MessageInfo,
    relayers: Vec<Addr>,
) -> StdResult<Response> {
    let api = deps.api;
    let owner_addr = read_owner_store(deps.storage).load()?;
    if api.addr_canonicalize(&info.sender.as_str())? != owner_addr {
        return Err(StdError::generic_err("NOT_AUTHORIZED"));
    }

    let mut relayers_store = relayers_store(deps.storage);
    for relayer in relayers {
        relayers_store.set(
            relayer.to_string().as_bytes(),
            &bincode::serialize(&true).unwrap(),
        );
    }

    Ok(Response::default())
}

pub fn try_remove_relayers(
    deps: DepsMut,
    info: MessageInfo,
    relayers: Vec<Addr>,
) -> StdResult<Response> {
    let api = deps.api;
    let owner_addr = read_owner_store(deps.storage).load()?;
    if api.addr_canonicalize(&info.sender.as_str())? != owner_addr {
        return Err(StdError::generic_err("NOT_AUTHORIZED"));
    }

    let mut relayers_store = relayers_store(deps.storage);
    for relayer in relayers {
        relayers_store.set(
            relayer.to_string().as_bytes(),
            &bincode::serialize(&false).unwrap(),
        );
    }

    Ok(Response::default())
}

pub fn try_relay(
    deps: DepsMut,
    info: MessageInfo,
    symbols: Vec<String>,
    rates: Vec<Uint128>,
    resolve_time: u64,
    request_id: u64,
) -> StdResult<Response> {
    let size = symbols.len();
    if !(rates.len() == size) {
        return Err(StdError::generic_err("NOT_ALL_INPUT_SIZES_ARE_THE_SAME"));
    }

    let is_relayer = query_is_relayer(deps.as_ref(), info.sender.clone())?;
    if !is_relayer {
        return Err(StdError::generic_err("NOT_A_RELAYER"));
    }

    let mut refs = ref_data_store(deps.storage);
    for (s, r) in zip!(&symbols, &rates) {
        let ref_data = refs
            .get(&s.as_bytes())
            .map(|b| bincode::deserialize::<RefData>(&b).unwrap());

        if ref_data.is_none() || resolve_time > ref_data.unwrap().resolve_time {
            refs.set(
                s.as_bytes(),
                &bincode::serialize(&RefData::new(r, resolve_time, request_id)).unwrap(),
            );
        }
    }

    Ok(Response::default())
}

pub fn try_force_relay(
    deps: DepsMut,
    info: MessageInfo,
    symbols: Vec<String>,
    rates: Vec<Uint128>,
    resolve_time: u64,
    request_id: u64,
) -> StdResult<Response> {
    let size = symbols.len();
    if !(rates.len() == size) {
        return Err(StdError::generic_err("NOT_ALL_INPUT_SIZES_ARE_THE_SAME"));
    }

    let is_relayer = query_is_relayer(deps.as_ref(), info.sender.clone())?;
    if !is_relayer {
        return Err(StdError::generic_err("NOT_A_RELAYER"));
    }

    let mut refs = ref_data_store(deps.storage);
    for (s, r) in zip!(&symbols, &rates) {
        refs.set(
            s.as_bytes(),
            &bincode::serialize(&RefData::new(r, resolve_time, request_id)).unwrap(),
        );
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => unwrap_query!(query_owner(deps), "SERIALIZE_OWNER_ERROR"),
        QueryMsg::IsRelayer { relayer } => {
            unwrap_query!(query_is_relayer(deps, relayer), "SERIALIZE_RELAYER_ERROR")
        }
        QueryMsg::GetRef { symbol } => {
            unwrap_query!(query_ref(deps, symbol), "SERIALIZE_REF_DATA_ERROR")
        }
        QueryMsg::GetReferenceData {
            base_symbol,
            quote_symbol,
        } => unwrap_query!(
            query_reference_data(deps, base_symbol, quote_symbol),
            "SERIALIZE_REFERENCE_DATA_ERROR"
        ),
        QueryMsg::GetReferenceDataBulk {
            base_symbols,
            quote_symbols,
        } => unwrap_query!(
            query_reference_data_bulk(deps, base_symbols, quote_symbols,),
            "SERIALIZE_REFERENCE_DATA_BULK_ERROR"
        ),
    }
}

fn query_owner(deps: Deps) -> StdResult<Addr> {
    read_owner_store(deps.storage)
        .load()
        .map(|ca| deps.api.addr_humanize(&ca).unwrap())
        .map_err(|_| StdError::generic_err("OWNER_NOT_INITIALIZED"))
}

fn query_is_relayer(deps: Deps, relayer: Addr) -> StdResult<bool> {
    match read_relayers_store(deps.storage).get(&relayer.as_bytes()) {
        Some(data) => Ok(bincode::deserialize(&data).unwrap()),
        _ => Ok(false),
    }
}

fn query_ref(deps: Deps, symbol: String) -> StdResult<RefData> {
    if symbol == String::from("USD") {
        return Ok(RefData::new(Uint128::new(E9), u64::MAX, 0));
    }
    match read_ref_data_store(deps.storage).get(&symbol.as_bytes()) {
        Some(data) => {
            let r: RefData = bincode::deserialize(&data).unwrap();
            if r.rate == Uint128::zero() || r.resolve_time == 0 {
                return Err(StdError::generic_err(format!(
                    "REF_DATA_NOT_AVAILABLE_FOR_KEY:{}",
                    symbol
                )));
            }
            Ok(r)
        }
        _ => Err(StdError::generic_err(format!(
            "REF_DATA_NOT_AVAILABLE_FOR_KEY:{}",
            symbol
        ))),
    }
}

fn query_reference_data(
    deps: Deps,
    base_symbol: String,
    quote_symbol: String,
) -> StdResult<ReferenceData> {
    let base = unwrap_or_return_err!(query_ref(deps, base_symbol));
    let quote = unwrap_or_return_err!(query_ref(deps, quote_symbol));
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
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coins, from_binary, Coin, OwnedDeps, StdError, Timestamp};

    fn init_msg() -> InstantiateMsg {
        InstantiateMsg {}
    }

    fn execute_transfer_ownership(o: &str) -> ExecuteMsg {
        ExecuteMsg::TransferOwnership {
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

    fn query_owner_msg() -> QueryMsg {
        QueryMsg::Owner {}
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
        match query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!("OWNER_NOT_INITIALIZED", msg),
            _ => panic!("Test Fail: expect OWNER_NOT_INITIALIZED"),
        }

        // Check if successfully set owner
        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // should be able to query
        let encoded_owner = query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap();
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
            from_binary::<Addr>(
                &query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap()
            ).unwrap()
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
            from_binary::<Addr>(
                &query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap()
            )
            .unwrap()
        );

        // should successfully set new owner
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                execute_transfer_ownership("new_owner")
            )
            .unwrap()
            .messages
            .len()
        );

        // check owner in the state should be new_owner
        assert_eq!(
            String::from("new_owner"),
            from_binary::<Addr>(
                &query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap()
            )
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
                execute_add_relayer(test_relayers.clone())
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
                execute_remove_relayer(test_relayers.clone())
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
                execute_add_relayer(vec!["owner"])
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
                        Uint128::from(3 * E9)
                    ],
                    100,
                    1
                )
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
                        Uint128::from(6 * E9)
                    ],
                    110,
                    1
                )
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
                execute_add_relayer(vec!["owner"])
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
                    1
                )
            )
            .unwrap()
            .messages
            .len()
        );

        // should successfully relay second time
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
                        Uint128::from(6 * E9)
                    ],
                    90,
                    1
                )
            )
            .unwrap()
            .messages
            .len()
        );

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

        // check ref data is correct. C should be data from the second time.
        assert_eq!(
            RefData::new(Uint128::from(6 * E9), 90, 1),
            from_binary::<RefData>(
                &query(deps.as_ref(), env.clone(), query_ref_data_msg("C".into())).unwrap()
            )
            .unwrap()
        );
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
                execute_add_relayer(vec!["owner"])
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
                    1
                )
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
                        Uint128::from(6 * E9)
                    ],
                    90,
                    1
                )
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
                execute_add_relayer(vec!["owner"])
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
                        Uint128::from(3 * E9)
                    ],
                    100,
                    1
                )
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
                    query_reference_data_msg("A".into(), "B".into())
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
                    query_reference_data_msg("A".into(), "C".into())
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
                    query_reference_data_msg("A".into(), "USD".into())
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
                execute_add_relayer(vec!["owner"])
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
                        Uint128::from(3 * E9)
                    ],
                    100,
                    1
                )
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
                ReferenceData::new(Uint128::from(3 * E9 * E9 / 2), 100, 100)
            ],
            from_binary::<Vec<ReferenceData>>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    query_reference_data_bulk_msg(
                        vec!["A".into(), "B".into(), "C".into()],
                        vec!["C".into(), "USD".into(), "B".into()]
                    )
                )
                .unwrap()
            )
            .unwrap()
        );
    }
}
