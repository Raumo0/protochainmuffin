use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, DepsMut, IbcEndpoint};
use cw_storage_plus::{Item};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::ContractError;
use crate::msg::TwapSettings;

/// static info on one channel that doesn't change
pub const CHANNEL_INFO: Item<ChannelInfo> = Item::new("channel_info");

pub const CONTRACT_STATE: Item<ContractInfo> = Item::new("contract_state");

pub const TWAP_SETTINGS: Item<TwapSettings> = Item::new("twap_settings");

#[cw_serde]
pub struct ChannelInfo {
    /// id of this channel
    pub id: String,
    /// the remote channel/port we connect to
    pub counterparty_endpoint: IbcEndpoint,
    /// the connection this exists on (you can use to query client/consensus info)
    pub connection_id: String,
}

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Eq, JsonSchema)]
pub enum StateStatus {
    #[default]
    None,
    Initialized,
    Requested,
    Processing,
    Paid,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Eq, JsonSchema)]
pub struct ContractInfo {
    pub owner_address: String,
    pub fixed_amount: Coin,
    pub contract_denom: String,
    pub state: StateStatus,
}

// Function to update the state status in ContractInfo
pub fn update_contract_status(deps: DepsMut,
                              new_status: StateStatus
) -> Result<(), ContractError> {
    // Load the current contract info from storage
    let mut contract_info = CONTRACT_STATE.load(deps.storage)?;

    // Update the state status
    contract_info.state = new_status;

    // Save the updated contract info back to storage
    CONTRACT_STATE.save(deps.storage, &contract_info)?;

    // Return a response with an action indicating the state has been updated
    Ok(())
}