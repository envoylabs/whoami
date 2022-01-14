mod contract_tests;
mod error;
pub mod execute;
pub mod msg;
pub mod query;
pub mod state;
pub mod utils;

use cosmwasm_std::{to_binary, Empty};

use execute::{
    burn, execute_instantiate, mint, mint_path, send_nft, set_admin_address,
    set_username_length_cap, transfer_nft, update_metadata, update_minting_fees,
    update_primary_alias,
};
use query::{
    contract_info, get_base_tokens_for_owner, get_parent_id, get_parent_nft_info, get_path,
    get_paths_for_owner, get_paths_for_owner_and_token, is_contract, primary_alias,
};

pub use crate::msg::{ExecuteMsg, Extension, InstantiateMsg, QueryMsg};

pub use crate::error::ContractError;

pub type Cw721MetadataContract<'a> = cw721_base::Cw721Contract<'a, Extension, Empty>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        let tract = Cw721MetadataContract::default();
        execute_instantiate(tract, deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        let tract = Cw721MetadataContract::default();
        match msg {
            ExecuteMsg::UpdateMintingFees(msg) => update_minting_fees(tract, deps, env, info, msg),
            ExecuteMsg::UpdateUsernameLengthCap { new_length } => {
                set_username_length_cap(tract, deps, env, info, new_length)
            }
            ExecuteMsg::Mint(msg) => mint(tract, deps, env, info, msg),
            ExecuteMsg::MintPath(msg) => mint_path(tract, deps, env, info, msg),
            ExecuteMsg::UpdateMetadata(msg) => update_metadata(tract, deps, env, info, msg),
            ExecuteMsg::UpdatePrimaryAlias { token_id } => {
                update_primary_alias(tract, deps, env, info, token_id)
            }
            // this actually sets the minter field,
            // but the interface is that we call it an admin_address
            ExecuteMsg::SetAdminAddress { admin_address } => {
                set_admin_address(tract, deps, env, info, admin_address)
            }
            // we override these purely because in each one of these cases
            // we want to remove any preferred username entries
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => transfer_nft(tract, deps, env, info, recipient, token_id),
            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => send_nft(tract, deps, env, info, contract, token_id, msg),
            ExecuteMsg::Burn { token_id } => burn(tract, deps, env, info, token_id),

            _ => tract
                .execute(deps, env, info, msg.into())
                .map_err(ContractError::Base),
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        let tract = Cw721MetadataContract::default();

        match msg {
            QueryMsg::BaseTokens {
                owner,
                start_after,
                limit,
            } => to_binary(&get_base_tokens_for_owner(
                tract,
                deps,
                owner,
                start_after,
                limit,
            )?),
            QueryMsg::PrimaryAlias { address } => {
                to_binary(&primary_alias(tract, deps, env, address)?)
            }
            QueryMsg::ContractInfo {} => to_binary(&contract_info(deps)?),
            QueryMsg::IsContract { token_id } => to_binary(&is_contract(tract, deps, token_id)?),
            QueryMsg::GetParentId { token_id } => to_binary(&get_parent_id(tract, deps, token_id)?),
            QueryMsg::GetParentInfo { token_id } => {
                to_binary(&get_parent_nft_info(tract, deps, token_id)?)
            }
            QueryMsg::GetFullPath { token_id } => to_binary(&get_path(tract, deps, token_id)?),
            QueryMsg::Paths {
                owner,
                start_after,
                limit,
            } => to_binary(&get_paths_for_owner(
                tract,
                deps,
                owner,
                start_after,
                limit,
            )?),
            QueryMsg::PathsForToken {
                owner,
                token_id,
                start_after,
                limit,
            } => to_binary(&get_paths_for_owner_and_token(
                deps,
                owner,
                token_id,
                start_after,
                limit,
            )?),
            _ => tract.query(deps, env, msg.into()).map_err(|err| err),
        }
    }
}
