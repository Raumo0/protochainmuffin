use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::AbciQueryRequest;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, IbcMsg, MessageInfo, Response, StdError, StdResult, to_json_binary};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;
use prost::Message;

use crate::error::ContractError;
use crate::helpers::query_balance;
use crate::msg::{ArithmeticTwapToNowRequest, CosmosQuery, ExecuteMsg, InstantiateMsg, InterchainQueryPacketData, QueryMsg, Timestamp};
use crate::state::{CHANNEL_INFO, CONTRACT_STATE, ContractInfo, StateStatus, TWAP_SETTINGS, update_contract_status};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:muffin";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract_info = ContractInfo {
        owner_address: msg.owner_address,
        fixed_amount: msg.fixed_amount,
        contract_denom: msg.contract_denom,
        state: StateStatus::Initialized,
    };

    CONTRACT_STATE.save(deps.storage, &contract_info)?;
    TWAP_SETTINGS.save(deps.storage, &msg.twap_request_info)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ClaimProposalAmount {} => claim_proposal_amount(deps, env),
    }
}

pub fn claim_proposal_amount(mut deps: DepsMut,
                             env: Env,
) -> Result<Response, ContractError> {
    let contract_info = CONTRACT_STATE.load(deps.storage)?;

    let contract_balance = query_balance(deps.as_ref(),
                                         env.contract.address.to_string(),
                                         contract_info.contract_denom.to_string())?;

    if contract_balance.amount.is_zero() {
        return Err(ContractError::Std(StdError::generic_err("Contract does not have any tokens to send it to owner")));
    }

    update_contract_status(deps.branch(), StateStatus::Requested)?;

    send_twap_icq_query(deps, env)
}

fn send_twap_icq_query(deps: DepsMut,
                       env: Env
) -> Result<Response, ContractError> {
    let channel_id: String = get_channel_id(deps.as_ref())?;

    let block_time = env.block.time;   // Get the current block time
    let four_hours = 4 * 3600;              // 4 hours in seconds

    // Subtract 4 hours from the block time
    let new_time = block_time.seconds() - four_hours;

    // Create the prost::Timestamp struct
    let timestamp = Timestamp {
        seconds: new_time as i64,
        nanos: 0,
    };

    let twap_settings = TWAP_SETTINGS.load(deps.as_ref().storage)?;

    let query_twap_request: ArithmeticTwapToNowRequest = ArithmeticTwapToNowRequest {
        pool_id: twap_settings.pool_id,
        base_asset: twap_settings.base_asset,
        quote_asset: twap_settings.quote_asset,
        start_time: Some(timestamp),
    };

    let req: AbciQueryRequest = AbciQueryRequest {
        data: query_twap_request.encode_to_vec(),
        path: "/osmosis.twap.v1beta1.Query/ArithmeticTwapToNow".to_string(),
        height: 0,
        prove: false,
    };

    let cosmos_query: CosmosQuery = CosmosQuery {
        requests: vec![req]
    };

    let packet_data: InterchainQueryPacketData = InterchainQueryPacketData {
        data: cosmos_query.encode_to_vec(),
        memo: "TWAP ICQ request".to_string(),
    };

    // timeout is in nanoseconds
    let timeout = env.block.time.plus_seconds(120);

    // prepare ibc message
    let ibc_msg = IbcMsg::SendPacket {
        channel_id: channel_id.clone(),
        data: to_json_binary(&packet_data)?,
        timeout: timeout.into(),
    };

    Ok(Response::new()
        .add_attribute("method", "send_query_balance")
        .add_attribute("channel", channel_id)
        .add_message(ibc_msg))
}

fn get_channel_id(deps: Deps) -> StdResult<String> {
    match CHANNEL_INFO.may_load(deps.storage)? {
        Some(channel_info) => Ok(channel_info.id), // Return the item if it's loaded
        None => Err(StdError::generic_err("Channel to ICQ module is not setup")),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ContractInfo {} => {
            let contract_info = CONTRACT_STATE.load(deps.storage)?;
            to_json_binary(&contract_info)
        }
    }
}

#[cfg(test)]
mod tests {}
