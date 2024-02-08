#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary,Addr,Uint128 , BankMsg, Coin, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;


use crate::error::ContractError;
use crate::msg::{CountResponse, WhitelistResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:counter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        count: msg.count,
        owner: info.sender.clone(),
        whitelist: msg.whitelist,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("count", msg.count.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Increment {} => try_increment(deps),
        ExecuteMsg::Reset { count } => try_reset(deps, info, count),
        ExecuteMsg::Add { whitelist } => try_add_whitelist(deps, info, whitelist),
        ExecuteMsg::Remove { whitelist } => try_remove_whitelist(deps, info, whitelist),
        ExecuteMsg::Distribute { amount, denom } => try_token_distribute(deps.as_ref(), _env, info, amount, &denom),
    }
}

pub fn try_increment(deps: DepsMut) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.count += 1;
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "try_increment"))
}

pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if info.sender != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.count = count;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "reset"))
}


pub fn try_add_whitelist(deps: DepsMut, info: MessageInfo , addresses:  Vec<String>) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if info.sender != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.whitelist.extend(addresses);
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "try_add_whitelist"))
}

pub fn try_remove_whitelist(deps: DepsMut, info: MessageInfo , addresses:  Vec<String>) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if info.sender != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.whitelist.retain(|address| !addresses.contains(address));
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "try_remove_whitelist"))
}

pub fn try_token_distribute(deps: Deps, env:Env, info: MessageInfo, amount: u128 , denom: &str) -> Result<Response, ContractError> { 
    if info.sender != STATE.load(deps.storage)?.owner {
        return Err(ContractError::Unauthorized {});
    }

    let whitelist_response: WhitelistResponse = query_whitelist(deps)?;
    let whitelist: Vec<String> = whitelist_response.whitelist;
    let whitelist_len = whitelist.len() as u128;
    let total_distribution_amount = amount * whitelist_len;
    
    
    let contract_address: Addr = env.contract.address.clone();
    // let denom = "token";
    
    // Query the balance of the contract address
    let contract_balance: Coin = deps.querier.query_balance(contract_address, denom)?;
    if contract_balance.amount < total_distribution_amount.into() {
        return Err(ContractError::InsufficientError {});
    }

    let mut messages: Vec<BankMsg> = vec![];
    for address in whitelist {
        let send_msg = BankMsg::Send {
            to_address: address.clone(),
            amount: vec![Coin {
                denom: denom.to_string(),
                amount: amount.into(),
            }],
        };
        messages.push(send_msg);
    }
    
    Ok(Response::new().add_attribute("method", "try_token_distribute"))

}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
        QueryMsg::GetWhitelist {} => to_binary(&query_whitelist(deps)?),
    }
}

fn query_count(deps: Deps) -> StdResult<CountResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(CountResponse { count: state.count })
}

fn query_whitelist(deps: Deps) -> StdResult<WhitelistResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(WhitelistResponse { whitelist: state.whitelist })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance,mock_dependencies_with_balances, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, QuerierWrapper, Uint128};

    #[test]
    fn proper_initialization_ops() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17 , whitelist: vec![] };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17, whitelist: vec![]  };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Increment {};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17, whitelist: vec![]  };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let unauth_info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // should now be 5
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }

    
    #[test]
    fn add_whitelist() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17, whitelist: vec![]  };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let addresses_to_add = vec!["address1".to_string(), "address2".to_string()];
        let add_result = try_add_whitelist(deps.as_mut(), info, addresses_to_add.clone());

        assert!(add_result.is_ok());
        // // beneficiary can release it
        // let info = mock_info("anyone", &coins(2, "token"));
        // let msg = ExecuteMsg::Increment {};
        // let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetWhitelist {}).unwrap();
        let value: WhitelistResponse = from_binary(&res).unwrap();
        assert_eq!(addresses_to_add, value.whitelist);
    }

    
    #[test]
    fn remove_whitelist() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg { count: 17, whitelist: vec!["address1".to_string(), "address2".to_string()]  };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let addresses_to_remove = vec!["address1".to_string()];
        let remove_result = try_remove_whitelist(deps.as_mut(), info, addresses_to_remove.clone());

        assert!(remove_result.is_ok());
        // // beneficiary can release it
        // let info = mock_info("anyone", &coins(2, "token"));
        // let msg = ExecuteMsg::Increment {};
        // let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetWhitelist {}).unwrap();
        let value: WhitelistResponse = from_binary(&res).unwrap();
        assert_eq!(vec!["address2".to_string()], value.whitelist);
    }
    
    #[test]
    fn test_token_distribute() {
        let mut deps = mock_dependencies_with_balance(&coins(1000, "token"));

        let msg_instantiate = InstantiateMsg { count: 17, whitelist: vec![]  };
        let info = mock_info("creator", &coins(1000, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg_instantiate).unwrap();

        let addresses_to_add = vec!["address1".to_string(), "address2".to_string()];
        let add_result = try_add_whitelist(deps.as_mut(), info.clone(), addresses_to_add.clone());
        
        assert!(add_result.is_ok());

        // log(&format!("Response: {:?}", add_result));
        
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetWhitelist {}).unwrap();
        let value: WhitelistResponse = from_binary(&res).unwrap();
        assert_eq!(addresses_to_add, value.whitelist);

        let sender = String::from("creator");
        let msg_distribute = ExecuteMsg::Distribute {amount: 100 , denom: "token".to_string()};
        // let _res1 = execute(deps.as_mut(), mock_env(), info, msg_distribute).unwrap();
        let _res1 = try_token_distribute(deps.as_ref(), mock_env(), info, 100 ,"token");
        assert!(_res1.is_ok());



        // // beneficiary can release it
        // let info = mock_info("anyone", &coins(2, "token"));
        // let msg = ExecuteMsg::Increment {};
        // let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();



        // let sender1 = String::from("address2");
        // let querier: QuerierWrapper = QuerierWrapper::new(&deps.querier);

        // let balance_sender1 = querier.query_balance(sender1, "token").unwrap();
        // assert_eq!(balance_sender1.amount, Uint128::new(100));

        // let balance_contract = querier.query_balance(mock_env().contract.address, "token").unwrap();
        // assert_eq!(balance_contract.amount, Uint128::new(5000));

        

        // let querier: QuerierWrapper = deps.querier;
        // let balance = querier.query_balance(sender, "token").unwrap();
        // assert_eq!(balance, Coin {
        //     denom: "token".to_string(),
        //     amount: Uint128::new(100), // 1000 - 100
        // });

        // // should increase counter by 1
        // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetWhitelist {}).unwrap();
        // let value: WhitelistResponse = from_binary(&res).unwrap();
        // assert_eq!(addresses_to_add, value.whitelist);
    }

    
}
