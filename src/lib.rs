mod contract_tests;
pub mod execute;
pub mod msg;
pub mod query;
pub mod state;

use cosmwasm_std::{to_binary, Empty};

pub use cw721_base::ContractError;

use execute::{
    burn, execute_instantiate, mint, send_nft, set_admin_address, transfer_nft, update_metadata,
    update_preferred_alias,
};
use query::{contract_info, preferred_alias};

pub use crate::msg::{ExecuteMsg, Extension, InstantiateMsg, QueryMsg};

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
            ExecuteMsg::Mint(msg) => mint(tract, deps, env, info, msg),
            ExecuteMsg::UpdateMetadata(msg) => update_metadata(tract, deps, env, info, msg),
            ExecuteMsg::UpdatePreferredAlias { token_id } => {
                update_preferred_alias(tract, deps, env, info, token_id)
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
                .map_err(|err| err),
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        let tract = Cw721MetadataContract::default();

        match msg {
            QueryMsg::PreferredAlias { address } => {
                to_binary(&preferred_alias(tract, deps, env, address)?)
            }
            QueryMsg::ContractInfo {} => to_binary(&contract_info(deps)?),
            _ => tract.query(deps, env, msg.into()).map_err(|err| err),
        }
    }
}
