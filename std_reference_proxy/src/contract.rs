use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{owner_store, read_owner_store, read_ref_contract_store, ref_contract_store};
use crate::struct_types::ReferenceData;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};

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
    msg: InstantiateMsg,
) -> StdResult<Response> {
    owner_store(deps.storage).save(&deps.api.addr_canonicalize(&info.sender.as_str())?)?;
    ref_contract_store(deps.storage)
        .save(&deps.api.addr_canonicalize(&msg.initial_ref.as_str())?)?;
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
        ExecuteMsg::SetRef { new_ref } => try_set_ref(deps, info, new_ref),
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

pub fn try_set_ref(deps: DepsMut, info: MessageInfo, new_ref: Addr) -> StdResult<Response> {
    let owner_addr = read_owner_store(deps.storage).load()?;
    if deps.api.addr_canonicalize(&info.sender.as_str())? != owner_addr {
        return Err(StdError::generic_err("NOT_AUTHORIZED"));
    }

    ref_contract_store(deps.storage).save(&deps.api.addr_canonicalize(&new_ref.as_str())?)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => unwrap_query!(query_owner(deps), "SERIALIZE_OWNER_ERROR"),
        QueryMsg::Ref {} => unwrap_query!(query_ref(deps), "SERIALIZE_REF_DATA_ERROR"),
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

fn query_ref(deps: Deps) -> StdResult<Addr> {
    read_ref_contract_store(deps.storage)
        .load()
        .map(|ca| deps.api.addr_humanize(&ca).unwrap())
        .map_err(|_| StdError::generic_err("REF_NOT_INITIALIZED"))
}

fn query_reference_data(
    deps: Deps,
    base_symbol: String,
    quote_symbol: String,
) -> StdResult<ReferenceData> {
    deps.querier.query_wasm_smart(
        &query_ref(deps)?,
        &QueryMsg::GetReferenceData {
            base_symbol,
            quote_symbol,
        },
    )
}

fn query_reference_data_bulk(
    deps: Deps,
    base_symbols: Vec<String>,
    quote_symbols: Vec<String>,
) -> StdResult<Vec<ReferenceData>> {
    deps.querier.query_wasm_smart(
        &query_ref(deps)?,
        &QueryMsg::GetReferenceDataBulk {
            base_symbols,
            quote_symbols,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coins, from_binary, Coin, StdError};
    use cosmwasm_std::{OwnedDeps, Timestamp};

    fn init_msg(r: &str) -> InstantiateMsg {
        InstantiateMsg {
            initial_ref: Addr::unchecked(r),
        }
    }

    fn handle_transfer_ownership(o: &str) -> ExecuteMsg {
        ExecuteMsg::TransferOwnership {
            new_owner: Addr::unchecked(o),
        }
    }

    fn handle_set_ref(r: &str) -> ExecuteMsg {
        ExecuteMsg::SetRef {
            new_ref: Addr::unchecked(r),
        }
    }

    fn query_owner_msg() -> QueryMsg {
        QueryMsg::Owner {}
    }

    fn query_ref_msg() -> QueryMsg {
        QueryMsg::Ref {}
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
        let msg = init_msg("test_ref");
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 789, 0);

        // owner not initialized yet
        match query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap_err() {
            StdError::GenericErr { msg, .. } => assert_eq!("OWNER_NOT_INITIALIZED", msg),
            _ => panic!("Test Fail: expect OWNER_NOT_INITIALIZED"),
        }

        // Check if successfully set owner
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Verify correct owner address
        let encoded_owner = query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap();
        assert_eq!(
            String::from("owner"),
            from_binary::<Addr>(&encoded_owner).unwrap()
        );

        // Verify correct ref address
        let encoded_ref_addr = query(deps.as_ref(), env.clone(), query_ref_msg()).unwrap();
        assert_eq!(
            String::from("test_ref"),
            from_binary::<Addr>(&encoded_ref_addr).unwrap()
        );
    }

    #[test]
    fn test_transfer_ownership_fail_unauthorized() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 789, 0);

        // should successfully instantiate owner
        assert_eq!(
            0,
            instantiate(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                init_msg("test_ref")
            )
            .unwrap()
            .messages
            .len()
        );

        // check owner in the state
        assert_eq!(
            String::from("owner"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap())
                .unwrap()
        );

        let (_, alice_env, alice_info) = get_mocks("alice", &coins(1000, "test_coin"), 789, 0);

        // should fail because sender is alice not owner
        match execute(
            deps.as_mut(),
            alice_env.clone(),
            alice_info.clone(),
            handle_transfer_ownership("new_owner"),
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
            instantiate(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                init_msg("test_ref")
            )
            .unwrap()
            .messages
            .len()
        );

        // // check owner in the state
        assert_eq!(
            String::from("owner"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap())
                .unwrap()
        );

        // should successfully set new owner
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                handle_transfer_ownership("new_owner")
            )
            .unwrap()
            .messages
            .len()
        );

        // check owner in the state should be new_owner
        assert_eq!(
            String::from("new_owner"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn test_set_ref_fail_unauthorized() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 789, 0);

        // should successfully instantiate owner
        assert_eq!(
            0,
            instantiate(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                init_msg("test_ref_1")
            )
            .unwrap()
            .messages
            .len()
        );

        // check owner in the state
        assert_eq!(
            String::from("owner"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap())
                .unwrap()
        );
        // check ref in the state
        assert_eq!(
            String::from("test_ref_1"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_ref_msg()).unwrap())
                .unwrap()
        );

        let (_, alice_env, alice_info) = get_mocks("alice", &coins(1000, "test_coin"), 789, 0);

        // should fail because sender is alice not owner
        match execute(
            deps.as_mut(),
            alice_env.clone(),
            alice_info.clone(),
            handle_set_ref("test_ref_1"),
        )
        .unwrap_err()
        {
            StdError::GenericErr { msg, .. } => assert_eq!("NOT_AUTHORIZED", msg),
            _ => panic!("Test Fail: expect NOT_AUTHORIZED"),
        }

        // check ref in the state
        assert_eq!(
            String::from("test_ref_1"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_ref_msg()).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn test_set_ref_success() {
        let (mut deps, env, info) = get_mocks("owner", &coins(1000, "test_coin"), 789, 0);

        // should successfully instantiate owner
        assert_eq!(
            0,
            instantiate(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                init_msg("test_ref_1")
            )
            .unwrap()
            .messages
            .len()
        );

        // check owner in the state
        assert_eq!(
            String::from("owner"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_owner_msg()).unwrap())
                .unwrap()
        );

        // check ref in the state
        assert_eq!(
            String::from("test_ref_1"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_ref_msg()).unwrap())
                .unwrap()
        );

        // should successfully set new owner
        assert_eq!(
            0,
            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                handle_set_ref("test_ref_2")
            )
            .unwrap()
            .messages
            .len()
        );

        // check owner in the state should be new_owner
        assert_eq!(
            String::from("test_ref_2"),
            from_binary::<Addr>(&query(deps.as_ref(), env.clone(), query_ref_msg()).unwrap())
                .unwrap()
        );
    }
}
