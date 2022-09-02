use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage,
};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::CONFIG;
use crate::struct_types::{Config, ReferenceData};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: info.sender,
        reference_contract: msg.reference_contract,
    };
    CONFIG.save(deps.storage, &config)?;
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
        ExecuteMsg::UpdateConfig {
            new_owner,
            new_reference_contract,
        } => execute_update_config(deps, info, new_owner, new_reference_contract),
    }
}

fn check_owner(storage: &dyn Storage, sender: &Addr) -> StdResult<()> {
    let config = CONFIG.load(storage)?;
    if *sender != config.owner {
        Err(StdError::generic_err("NOT_AUTHORIZED"))
    } else {
        Ok(())
    }
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Option<Addr>,
    new_reference_contract: Option<Addr>,
) -> StdResult<Response> {
    check_owner(deps.storage, &info.sender)?;

    let mut config = CONFIG.load(deps.storage)?;

    if let Some(owner) = new_owner {
        config.owner = owner;
    }

    if let Some(reference_contract) = new_reference_contract {
        config.reference_contract = reference_contract;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
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

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_reference_data(
    deps: Deps,
    base_symbol: String,
    quote_symbol: String,
) -> StdResult<ReferenceData> {
    deps.querier.query_wasm_smart(
        &query_config(deps)?.reference_contract,
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
        &query_config(deps)?.reference_contract,
        &QueryMsg::GetReferenceDataBulk {
            base_symbols,
            quote_symbols,
        },
    )
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    // This function will setup the contract for other tests
    fn setup(mut deps: DepsMut, sender: &str, contract: &str) {
        let info = mock_info(sender, &[]);
        let env = mock_env();
        let res = instantiate(
            deps.branch(),
            env,
            info,
            InstantiateMsg {
                reference_contract: Addr::unchecked(contract),
            },
        )
        .unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(
            query_config(deps.as_ref()).unwrap(),
            Config {
                owner: Addr::unchecked(sender),
                reference_contract: Addr::unchecked(contract),
            }
        )
    }

    mod instantiate {
        use super::*;

        #[test]
        fn can_instantiate() {
            let mut deps = mock_dependencies();
            let init_msg = InstantiateMsg {
                reference_contract: Addr::unchecked("standard_reference_basic"),
            };
            let info = mock_info("owner", &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, init_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                query_config(deps.as_ref()).unwrap(),
                Config {
                    owner: Addr::unchecked("owner"),
                    reference_contract: Addr::unchecked("standard_reference_basic"),
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
            setup(deps.as_mut(), "owner", "contract");

            // Test authorized attempt to update config
            let info = mock_info("owner", &[]);
            let env = mock_env();
            let msg = UpdateConfig {
                new_owner: Option::from(Addr::unchecked("new_owner")),
                new_reference_contract: Option::from(Addr::unchecked("contract")),
            };
            execute(deps.as_mut(), env, info, msg).unwrap();
            let config = query_config(deps.as_ref()).unwrap();
            assert_eq!(
                config,
                Config {
                    owner: Addr::unchecked("new_owner"),
                    reference_contract: Addr::unchecked("contract"),
                }
            );

            // Test attempt to partially update config
            let info = mock_info("new_owner", &[]);
            let env = mock_env();
            let msg = UpdateConfig {
                new_owner: Option::from(Addr::unchecked("owner")),
                new_reference_contract: None,
            };
            execute(deps.as_mut(), env, info, msg).unwrap();
            let config = query_config(deps.as_ref()).unwrap();
            assert_eq!(
                config,
                Config {
                    owner: Addr::unchecked("owner"),
                    reference_contract: Addr::unchecked("contract"),
                }
            );
        }

        #[test]
        fn cannot_update_config_by_others() {
            // Setup
            let mut deps = mock_dependencies();
            setup(deps.as_mut(), "owner", "contract");

            // Test unauthorized attempt to update config
            let info = mock_info("user", &[]);
            let env = mock_env();
            let msg = UpdateConfig {
                new_owner: Option::from(Addr::unchecked("user")),
                new_reference_contract: None,
            };
            let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(err, StdError::generic_err("NOT_AUTHORIZED"));
        }
    }
}
