use std::ops::Sub;
use std::str::FromStr;
use cosmwasm_std::{BankMsg, Binary, Coin, DepsMut, DistributionMsg, Env, from_json, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcOrder, IbcPacket, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, Reply, Response, StdError, SubMsg, SubMsgResult, Uint128};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use prost::Message;

use crate::{ContractError, error::Never};
use crate::ack::{Ack, make_ack_fail, make_ack_success};
use crate::helpers::query_balance;
use crate::msg::{ArithmeticTwapToNowResponse, CosmosResponse, CosmosResponsePacket, InterchainQueryPacketAck};
use crate::state::{CHANNEL_INFO, ChannelInfo, CONTRACT_STATE, StateStatus, update_contract_status};

pub const IBC_VERSION: &str = "icq-1";

/// Handles the `OpenInit` and `OpenTry` parts of the IBC handshake.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse, ContractError> {
    validate_order_and_version(msg.channel(), msg.counterparty_version())?;
    Ok(None)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    validate_order_and_version(msg.channel(), msg.counterparty_version())?;

    let channel: IbcChannel = msg.into();
    let info = ChannelInfo {
        id: channel.endpoint.channel_id,
        counterparty_endpoint: channel.counterparty_endpoint,
        connection_id: channel.connection_id,
    };
    CHANNEL_INFO.save(deps.storage, &info)?;

    Ok(IbcBasicResponse::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let channel = msg.channel().endpoint.channel_id.clone();
    // Reset the state for the channel.
    CHANNEL_INFO.remove(deps.storage);
    Ok(IbcBasicResponse::new()
        .add_attribute("method", "ibc_channel_close")
        .add_attribute("channel", channel))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    Ok(IbcReceiveResponse::new(make_ack_success()).add_attribute("method", "ibc_packet_receive"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let icq_msg: Ack = from_json(&msg.acknowledgement.data)?;
    match icq_msg {
        Ack::Result(_) => on_packet_success(deps, env, msg.acknowledgement.data, msg.original_packet),
        Ack::Error(error) => {
            update_contract_status(deps, StateStatus::Failed)?;
            Ok(IbcBasicResponse::new()
                   .add_attribute("method", "ibc_packet_ack")
                   .add_attribute("error", error.to_string())
                   .add_attribute("sequence", msg.original_packet.sequence.to_string()))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    Ok(IbcBasicResponse::new().add_attribute("method", "ibc_packet_timeout"))
}

pub fn validate_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    // We expect an unordered channel here. Ordered channels have the
    // property that if a message is lost the entire channel will stop
    // working until you start it again.
    if channel.order != IbcOrder::Unordered {
        return Err(ContractError::OnlyOrderedChannel {});
    }

    if channel.version != IBC_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            actual: channel.version.to_string(),
            expected: IBC_VERSION.to_string(),
        });
    }

    // Make sure that we're talking with a counterparty who speaks the
    // same "protocol" as us.
    //
    // For a connection between chain A and chain B being established
    // by chain A, chain B knows counterparty information during
    // `OpenTry` and chain A knows counterparty information during
    // `OpenAck`. We verify it when we have it but when we don't it's
    // alright.
    if let Some(counterparty_version) = counterparty_version {
        if counterparty_version != IBC_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                actual: counterparty_version.to_string(),
                expected: IBC_VERSION.to_string(),
            });
        }
    }

    Ok(())
}

fn on_packet_success(mut deps: DepsMut,
                     env: Env,
                     result: Binary,
                     packet: IbcPacket
) -> Result<IbcBasicResponse, ContractError> {
    update_contract_status(deps.branch(), StateStatus::Processing)?;

    let ack_data: InterchainQueryPacketAck = from_json(&result)?;

    let cosmos_response: CosmosResponsePacket = from_json(&ack_data.result)?;
    let query_responses: CosmosResponse = CosmosResponse::decode(cosmos_response.data.as_slice())?;
    let first_response = query_responses.responses.first().unwrap();

    let price_response: ArithmeticTwapToNowResponse = ArithmeticTwapToNowResponse::decode(first_response.value.as_slice())?;
    let arithmetic_twap = Uint128::from_str(&price_response.arithmetic_twap).map_err(|_| ContractError::ParseError)?;

    let contract_info = CONTRACT_STATE.load(deps.as_ref().storage)?;

    let required_balance = calculate_required_balance(
        contract_info.fixed_amount,
        arithmetic_twap,
        contract_info.contract_denom.to_string(),
    )?;

    let contract_balance = query_balance(deps.as_ref(),
                                         env.contract.address.to_string(),
                                         contract_info.contract_denom.to_string())?;

    if contract_balance.amount.lt(&required_balance.amount) {
        update_contract_status(deps.branch(), StateStatus::Failed)?;
        return Err(ContractError::NoEnoughTokens{});
    }

    // Create a BankMsg::Send message
    let send_tokens_to_contract_owner_msg = BankMsg::Send {
        to_address: contract_info.owner_address,
        amount: vec![required_balance.clone()],
    };

    let remain_balance: Coin = subtract_coins(contract_balance, required_balance)?;

    // Create a custom distribution message for donating to the community pool
    let send_tokens_to_community_pool_msg = DistributionMsg::FundCommunityPool {
      amount: vec![remain_balance]
    };

    let send_coin_to_user_sub_msg: SubMsg = SubMsg::reply_on_success(send_tokens_to_contract_owner_msg, RECEIVE_ID);
    let send_coin_to_community_sub_msg: SubMsg = SubMsg::reply_on_success(send_tokens_to_community_pool_msg, RECEIVE_ID);

    Ok(IbcBasicResponse::new()
        .add_submessage(send_coin_to_user_sub_msg)
        .add_submessage(send_coin_to_community_sub_msg)
        .add_attribute("method", "ibc_packet_ack")
        .add_attribute("sequence", packet.sequence.to_string())
    )
}

fn calculate_required_balance(
    fixed_amount: Coin,
    arithmetic_twap: Uint128,
    contract_denom: String,
) -> Result<Coin, ContractError> {
    // Perform the multiplication and check for overflow
    let required_amount_in_nano = fixed_amount
        .amount
        .checked_mul(arithmetic_twap)?;

    // Perform the division to adjust for the scale of twap_price (1_000_000_000_000_000_000)
    let required_amount = required_amount_in_nano
        .checked_div(Uint128::new(1_000_000_000_000_000_000))?;

    // Create the resulting Coin with the calculated amount and specified denomination
    let required_token = Coin {
        amount: required_amount,
        denom: contract_denom,
    };

    Ok(required_token)
}

fn subtract_coins(coin1: Coin, coin2: Coin) -> Result<Coin, ContractError> {
    // Ensure the denominations are the same
    if coin1.denom != coin2.denom {
        return Err(ContractError::Std(StdError::generic_err("Different denominations. Cannot subtract.")));
    }

    let total_amount = coin1.amount;
    let used_amount = coin2.amount;

    // Subtract the amounts
    let result_amount = total_amount.sub(used_amount);

    // Return the resulting coin with the same denomination
    Ok(Coin {
        denom: coin1.denom,
        amount: result_amount,
    })
}

const RECEIVE_ID: u64 = 1337;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        RECEIVE_ID => match reply.result {
            SubMsgResult::Ok(_) => {
                update_contract_status(deps, StateStatus::Paid)?;
                Ok(Response::new().add_attribute("method", "contract_paid"))
            },
            SubMsgResult::Err(err) => {
                update_contract_status(deps, StateStatus::Failed)?;
                Ok(Response::new().set_data(make_ack_fail(err)))
            }
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}
