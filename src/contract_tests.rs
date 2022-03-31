#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
mod tests {
    use crate::entry;

    use crate::utils::{
        is_path, namespace_in_path, pgp_pubkey_format_is_valid, remove_namespace_from_path,
        validate_path_characters, validate_username_characters,
    };

    use crate::error::ContractError;

    use crate::msg::{
        AddressOfResponse, ContractInfoResponse, ExecuteMsg, Extension, GetParentIdResponse,
        GetPathResponse, InstantiateMsg, IsContractResponse, Metadata, MintMsg,
        PrimaryAliasResponse, QueryMsg, SurchargeInfo, UpdateMetadataMsg, UpdateMintingFeesMsg,
        WhoamiNftInfoResponse,
    };
    use crate::Cw721MetadataContract;
    use cosmwasm_std::{
        coins, from_binary, to_binary, BankMsg, CosmosMsg, Decimal, DepsMut, Response, StdError,
        Uint128,
    };
    use cw721_base::MinterResponse;

    use cw721::{Cw721Query, NftInfoResponse, OwnerOfResponse, TokensResponse};

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    // test some utils first
    #[test]
    fn pgp_pubkey_validator() {
        // obviously this is not valid, but
        // we only check a very naive format
        let mock_pgp_key_format = "-----BEGIN PGP PUBLIC KEY BLOCK-----

mQINBFRUAGoBEACuk6ze2V2pZtScf1Ul25N2CX19AeL7sVYwnyrTYuWdG2FmJx4x
DLTLVUazp2AEm/JhskulL/7VCZPyg7ynf+o20Tu9/6zUD7p0rnQA2k3Dz+7dKHHh
eEsIl5EZyFy1XodhUnEIjel2nGe6f1OO7Dr3UIEQw5JnkZyqMcbLCu9sM2twFyfa
a8JNghfjltLJs3/UjJ8ZnGGByMmWxrWQUItMpQjGr99nZf4L+IPxy2i8O8WQewB5
fvfidBGruUYC+mTw7CusaCOQbBuZBiYduFgH8hRW97KLmHn0xzB1FV++KI7syo8q
XGo8Un24WP40IT78XjKO
=nUop
-----END PGP PUBLIC KEY BLOCK-----";

        let invalid_pubkey_format = "
mQINBFRUAGoBEACuk6ze2V2pZtScf1Ul25N2CX19AeL7sVYwnyrTYuWdG2FmJx4x
DLTLVUazp2AEm/JhskulL/7VCZPyg7ynf+o20Tu9/6zUD7p0rnQA2k3Dz+7dKHHh
eEsIl5EZyFy1XodhUnEIjel2nGe6f1OO7Dr3UIEQw5JnkZyqMcbLCu9sM2twFyfa
a8JNghfjltLJs3/UjJ8ZnGGByMmWxrWQUItMpQjGr99nZf4L+IPxy2i8O8WQewB5
fvfidBGruUYC+mTw7CusaCOQbBuZBiYduFgH8hRW97KLmHn0xzB1FV++KI7syo8q
XGo8Un24WP40IT78XjKO
=nUop
-----END PGP PUBLIC KEY BLOCK-----";

        let first_check = pgp_pubkey_format_is_valid(mock_pgp_key_format);
        assert_eq!(first_check, true);

        let second_check = pgp_pubkey_format_is_valid(invalid_pubkey_format);
        assert_eq!(second_check, false);
    }

    #[test]
    fn username_validator() {
        let first_check = validate_username_characters("jeffvader");
        assert_eq!(first_check, true);

        let second_check = validate_username_characters("jeff-vader");
        assert_eq!(second_check, true);

        let third_check = validate_username_characters("jeff--vader");
        assert_eq!(third_check, false);

        let fourth_check = validate_username_characters("_jeff-vader");
        assert_eq!(fourth_check, true);

        let fifth_check = validate_username_characters("jeff_vader");
        assert_eq!(fifth_check, true);

        let sixth_check = validate_username_characters("_jeff_vader");
        assert_eq!(sixth_check, true);

        let seventh_check = validate_username_characters("-jeff_vader");
        assert_eq!(seventh_check, true);

        let eighth_check = validate_username_characters("__jeffvader");
        assert_eq!(eighth_check, false);

        let ninth_check = validate_username_characters("j3ffv4d3r");
        assert_eq!(ninth_check, true);

        let tenth_check = validate_username_characters("j3ff_v4d3r");
        assert_eq!(tenth_check, true);

        let eleventh_check = validate_username_characters("j3ff__v4d3r");
        assert_eq!(eleventh_check, false);

        let twelfth_check = validate_username_characters("jeff_-vader");
        assert_eq!(twelfth_check, false);

        // strictly speaking these are invalid
        // but we should normalize before we even hit these
        let thirteenth_check = validate_username_characters("JeffVader");
        assert_eq!(thirteenth_check, false);

        let fourteenth_check = validate_username_characters("Jeff");
        assert_eq!(fourteenth_check, false);
    }

    #[test]
    fn path_validator() {
        let first_check = validate_path_characters("jeffvader", "death-star-employees");
        assert_eq!(first_check, true);

        let second_check =
            validate_path_characters("jeffvader-notable-works", "death-star-employees");
        assert_eq!(second_check, true);

        // if this were let through it would really screw things up
        let third_check = validate_path_characters("jeff::vader", "death-star-employees");
        assert_eq!(third_check, false);

        let fourth_check = validate_path_characters("jeff-vader", "death-star-employees");
        assert_eq!(fourth_check, true);

        let fifth_check = validate_path_characters("jeff_vader", "death-star-employees");
        assert_eq!(fifth_check, true);

        // no two special chars together
        let sixth_check =
            validate_path_characters("_jeff_vader/_past_employment", "death-star-employees");
        assert_eq!(sixth_check, false);

        // no leading, as it will result in same error case
        let seventh_check = validate_path_characters("-jeff_vader", "death-star-employees");
        assert_eq!(seventh_check, false);

        let eighth_check = validate_path_characters(
            "jeffvader/past-construction-projects//death-star-one",
            "death-star-employees",
        );
        assert_eq!(eighth_check, false);

        let ninth_check = validate_path_characters("j3ffv4d3r", "death-star-employees");
        assert_eq!(ninth_check, true);

        let tenth_check = validate_path_characters("j3ff_v4d3r", "death-star-employees");
        assert_eq!(tenth_check, true);

        let eleventh_check = validate_path_characters("j3ff__v4d3r", "death-star-employees");
        assert_eq!(eleventh_check, false);

        let twelfth_check = validate_path_characters("jeff_-vader", "death-star-employees");
        assert_eq!(twelfth_check, false);

        // strictly speaking these are invalid
        // but we should normalize before we even hit these
        let thirteenth_check = validate_path_characters("JeffVader", "death-star-employees");
        assert_eq!(thirteenth_check, false);

        // no trailing
        let fourteenth_check = validate_path_characters("jeffvader-", "death-star-employees");
        assert_eq!(fourteenth_check, false);

        let fifteenth_check = validate_path_characters(
            "jeff/vader/trying/to/screw/up/parsing/",
            "death-star-employees",
        );
        assert_eq!(fifteenth_check, false);

        // just like Just A Minute
        // there will be no - well
        // maybe deviation and hesitation
        // but no repetition
        let sixteenth_check = validate_path_characters("jeff-vader-trying-his-best", "jeff-vader");
        assert_eq!(sixteenth_check, false);

        let seventeenth_check =
            validate_path_characters("trying-his-best-it-is-jeff-vader", "jeff-vader");
        assert_eq!(seventeenth_check, false);
    }

    // reminder: a token_id (i.e. path) can only ever be namespaced under its parent
    // so ids like jeff/vader::path/goes/here are not possible
    // even if you _might_ see that as a fully-qualified path iif they own jeff & vader
    // and have set jeff as the parent of vader. _phew_
    #[test]
    fn remove_namespace_from_path_test() {
        // token id called is employment
        let first_check = remove_namespace_from_path("jeffvader::employment", "jeffvader");
        assert_eq!(first_check, "::employment");

        // token id called is notable-works
        let second_check = remove_namespace_from_path(
            "jeffvader::notable-works/star-wars/a-new-hope",
            "jeffvader",
        );
        assert_eq!(second_check, "::notable-works/star-wars/a-new-hope");

        // token id called is employment
        // should be no-op
        let third_check = remove_namespace_from_path("employment", "vader");
        assert_eq!(third_check, "employment");

        let fourth_check =
            remove_namespace_from_path("jeffvader::employment/death-star-1", "jeffvader");
        assert_eq!(fourth_check, "::employment/death-star-1");
    }

    #[test]
    fn is_path_test() {
        // token id called is employment
        let first_check = is_path("jeffvader::employment");
        assert_eq!(first_check, true);

        // token id called is notable-works
        let second_check = is_path("jeffvader::notable-works/star-wars/a-new-hope");
        assert_eq!(second_check, true);

        // token id called is employment
        let third_check = is_path("jeff/vader::employment");
        assert_eq!(third_check, true);

        let fourth_check = is_path("jeffvader/employment/death-star-1");
        assert_eq!(fourth_check, false);
    }

    // reminder: a token_id (i.e. path) can only ever be namespaced under its parent
    // so ids like jeff/vader::path/goes/here are not possible
    // even if you _might_ see that as a fully-qualified path iif they own jeff & vader
    // and have set jeff as the parent of vader. _phew_
    #[test]
    fn namespace_in_path_test() {
        // token id called is employment
        let first_check = namespace_in_path("jeffvader::employment", "jeffvader");
        assert_eq!(first_check, true);

        // token id called is notable-works
        let second_check =
            namespace_in_path("jeffvader::notable-works/star-wars/a-new-hope", "jeffvader");
        assert_eq!(second_check, true);

        // token id called is employment
        let third_check = namespace_in_path("vader::employment", "vader");
        assert_eq!(third_check, true);

        let fourth_check = namespace_in_path("jeffvader::employment/death-star-1", "vader");
        assert_eq!(fourth_check, false);

        let fifth_check = namespace_in_path("jeffvader::employment/death-star-1", "yoda");
        assert_eq!(fifth_check, false);
    }

    const CREATOR: &str = "creator";
    const MINTER: &str = "jeff-vader";
    const CONTRACT_NAME: &str = "whoami";
    const SYMBOL: &str = "WHO";

    fn setup_contract(deps: DepsMut<'_>) -> Cw721MetadataContract<'static> {
        let contract = Cw721MetadataContract::default();
        let msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: None,
            admin_address: String::from(MINTER),
            username_length_cap: None,
        };
        let info = mock_info("creator", &[]);
        let res = entry::instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        contract
    }

    #[test]
    fn update_minting_fees_with_base_fee() {
        let mut deps = mock_dependencies();

        let jeff_address = "jeff-addr".to_string();

        let info = mock_info(&jeff_address, &[]);
        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: None,
            burn_percentage: None,
            short_name_surcharge: None,
            admin_address: jeff_address,
            username_length_cap: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // Note that this is a forced upsert, so fields should be declared as exactly
        // the desired values, they are not merged
        // this means fields that are a Some can actually be updated to be None later
        let mint_msg = UpdateMintingFeesMsg {
            base_mint_fee: Some(Uint128::new(1000000)),
            burn_percentage: None,
            token_cap: None,
            short_name_surcharge: None,
        };
        let exec_msg = ExecuteMsg::UpdateMintingFees(mint_msg);
        entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let contract_query_res: ContractInfoResponse = from_binary(
            &entry::query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap(),
        )
        .unwrap();

        let expected_res = ContractInfoResponse {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: None, // THIS IS IMPORTANT
            base_mint_fee: Some(Uint128::new(1000000)),
            burn_percentage: None,
            short_name_surcharge: None,
        };

        assert_eq!(contract_query_res, expected_res);
    }

    #[test]
    fn update_minting_fees_with_base_fee_and_surcharge() {
        let mut deps = mock_dependencies();

        let jeff_address = "jeff-addr".to_string();

        let info = mock_info(&jeff_address, &[]);
        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: None,
            burn_percentage: None,
            short_name_surcharge: None,
            admin_address: jeff_address,
            username_length_cap: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // Note that this is a forced upsert, so fields should be declared as exactly
        // the desired values, they are not merged
        // this means fields that are a Some can actually be updated to be None later
        let mint_msg = UpdateMintingFeesMsg {
            base_mint_fee: Some(Uint128::new(1000000)),
            burn_percentage: None,
            token_cap: Some(3),
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5,
                surcharge_fee: Uint128::new(2000000),
            }),
        };
        let exec_msg = ExecuteMsg::UpdateMintingFees(mint_msg);
        entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let contract_query_res: ContractInfoResponse = from_binary(
            &entry::query(deps.as_ref(), mock_env(), QueryMsg::ContractInfo {}).unwrap(),
        )
        .unwrap();

        let expected_res = ContractInfoResponse {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(3),
            base_mint_fee: Some(Uint128::new(1000000)),
            burn_percentage: None,
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5,
                surcharge_fee: Uint128::new(2000000),
            }),
        };

        assert_eq!(contract_query_res, expected_res);
    }

    #[test]
    fn update_username_length_cap_from_default() {
        let contract = Cw721MetadataContract::default();
        let mut deps = mock_dependencies();

        let jeff_address = "jeff-addr".to_string();
        let john_address = "john-q-rando-addr".to_string();

        let jeff_sender_info = mock_info(&jeff_address, &[]);
        let john_sender_info = mock_info(&john_address, &[]);

        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: None,
            burn_percentage: None,
            short_name_surcharge: None,
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            init_msg,
        )
        .unwrap();

        // CHECK: john cannot update
        let john_failed_attempt_1 = entry::execute(
            deps.as_mut(),
            mock_env(),
            john_sender_info,
            ExecuteMsg::UpdateUsernameLengthCap { new_length: 25 },
        )
        .unwrap_err();
        assert_eq!(john_failed_attempt_1, ContractError::Unauthorized {});

        // CHECK: can't mint 21 chr NFT
        let token_id = "jeffisthebest12345678".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();

        let meta = Metadata {
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id,
            owner: jeff_address,
            token_uri: Some(token_uri),
            extension: meta,
        });

        let failed_mint = entry::execute(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            mint_msg.clone(),
        )
        .unwrap_err();
        assert_eq!(failed_mint, ContractError::TokenNameInvalid {});

        // CHECK: jeff can update length cap
        let _ = entry::execute(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            ExecuteMsg::UpdateUsernameLengthCap { new_length: 25 },
        );

        // CHECK: minting is back on the menu boys
        let _ = entry::execute(deps.as_mut(), mock_env(), jeff_sender_info, mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);
    }

    #[test]
    fn update_username_length_cap() {
        let contract = Cw721MetadataContract::default();
        let mut deps = mock_dependencies();

        let jeff_address = "jeff-addr".to_string();

        let jeff_sender_info = mock_info(&jeff_address, &[]);

        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: None,
            burn_percentage: None,
            short_name_surcharge: None,
            admin_address: jeff_address.clone(),
            username_length_cap: Some(22),
        };
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            init_msg,
        )
        .unwrap();

        // CHECK: CAN mint 21 chr NFT
        let token_id = "jeffisthebest12345678".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();

        let meta = Metadata {
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        let _ = entry::execute(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            mint_msg,
        )
        .unwrap();

        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        let info = contract.nft_info(deps.as_ref(), token_id).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: meta.clone(),
            }
        );

        // CHECK: but cannot mint 25 chr NFT
        let token_id_2 = "jeffisthebestest123456789".to_string();
        let mint_msg_2 = ExecuteMsg::Mint(MintMsg {
            token_id: token_id_2.clone(),
            owner: jeff_address,
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        let failed_mint = entry::execute(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            mint_msg_2.clone(),
        )
        .unwrap_err();
        assert_eq!(failed_mint, ContractError::TokenNameInvalid {});

        // CHECK: jeff can update length cap
        let _ = entry::execute(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            ExecuteMsg::UpdateUsernameLengthCap { new_length: 25 },
        );

        // CHECK: can mint longer name now
        let _ = entry::execute(deps.as_mut(), mock_env(), jeff_sender_info, mint_msg_2).unwrap();

        // CHECK: ensure num tokens increases
        let count2 = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(2, count2.count);

        let info_res_2 = contract.nft_info(deps.as_ref(), token_id_2).unwrap();
        assert_eq!(
            info_res_2,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: meta,
            }
        );
    }

    #[test]
    fn update_admin_address() {
        let mut deps = mock_dependencies();

        let jeff_address = "jeff-addr".to_string();
        let john_address = "john-q-rando-addr".to_string();

        let jeff_sender_info = mock_info(&jeff_address, &[]);
        let john_sender_info = mock_info(&john_address, &[]);

        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: None,
            burn_percentage: None,
            short_name_surcharge: None,
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            init_msg,
        )
        .unwrap();

        // CHECK: john cannot update
        let john_failed_attempt_1 = entry::execute(
            deps.as_mut(),
            mock_env(),
            john_sender_info.clone(),
            ExecuteMsg::SetAdminAddress {
                admin_address: john_address.clone(),
            },
        )
        .unwrap_err();
        assert_eq!(john_failed_attempt_1, ContractError::Unauthorized {});

        // CHECK: but jeff can
        let _ = entry::execute(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            ExecuteMsg::SetAdminAddress {
                admin_address: john_address.clone(),
            },
        );

        let contract_query_res1: MinterResponse = from_binary(
            &entry::query(deps.as_ref(), mock_env(), QueryMsg::AdminAddress {}).unwrap(),
        )
        .unwrap();

        let expected_res1 = MinterResponse {
            minter: john_address,
        };

        assert_eq!(contract_query_res1, expected_res1);

        // CHECK: now jeff cannot
        let jeff_failed_attempt_1 = entry::execute(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info,
            ExecuteMsg::SetAdminAddress {
                admin_address: jeff_address.clone(),
            },
        )
        .unwrap_err();
        assert_eq!(jeff_failed_attempt_1, ContractError::Unauthorized {});

        // CHECK but john can
        let _ = entry::execute(
            deps.as_mut(),
            mock_env(),
            john_sender_info,
            ExecuteMsg::SetAdminAddress {
                admin_address: jeff_address.clone(),
            },
        );

        let contract_query_res2: MinterResponse = from_binary(
            &entry::query(deps.as_ref(), mock_env(), QueryMsg::AdminAddress {}).unwrap(),
        )
        .unwrap();

        let expected_res2 = MinterResponse {
            minter: jeff_address,
        };

        assert_eq!(contract_query_res2, expected_res2);
    }

    #[test]
    fn base_minting_underneath_path() {
        let jeff_address = String::from("jeff-vader");
        let allowed = mock_info(&jeff_address, &[]);
        let mut deps = mock_dependencies();

        let contract = setup_contract(deps.as_mut());
        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(5),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: None,
            admin_address: String::from(MINTER),
            username_length_cap: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), allowed.clone(), init_msg).unwrap();

        // init a plausible username
        let token_id = "jeff".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();

        let meta = Metadata {
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // CHECK: jeff can mint
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // CHECK: this nft info is correct
        let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: meta,
            }
        );

        // CHECK: owner info is correct
        let owner = contract
            .owner_of(deps.as_ref(), mock_env(), token_id.clone(), true)
            .unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: String::from("jeff-vader"),
                approvals: vec![],
            }
        );

        // CHECK: mint a path
        let path_id = "vehicles".to_string();
        let path_uri = "https://example.com/jeff-vader/vehicles".to_string();

        let path_meta = Metadata {
            parent_token_id: Some(token_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(path_uri.clone()),
            extension: path_meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id = format!("{}::{}", token_id, path_id);
        assert_eq!(prepended_path_id, "jeff::vehicles");

        // CHECK: this path info is correct
        let path_info = contract
            .nft_info(deps.as_ref(), prepended_path_id.clone())
            .unwrap();
        assert_eq!(
            path_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(path_uri),
                extension: path_meta,
            }
        );

        // CHECK: cannot mint token under path
        let meta_2 = Metadata {
            parent_token_id: Some(prepended_path_id),
            ..Metadata::default()
        };
        let mint_msg2 = ExecuteMsg::Mint(MintMsg {
            token_id: "tie-fighter".to_string(),
            owner: jeff_address,
            token_uri: Some("https://example.com/tie-fighter".to_string()),
            extension: meta_2,
        });

        // CHECK: result is an err
        let err = entry::execute(deps.as_mut(), mock_env(), allowed, mint_msg2).unwrap_err();
        assert_eq!(err, ContractError::CycleDetected {});

        // CHECK: ensure num tokens does not increase
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(2, count.count);
    }

    #[test]
    fn base_minting() {
        let mut deps = mock_dependencies();
        let contract = setup_contract(deps.as_mut());

        // init a plausible username
        let token_id = "jeffisthebest".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // CHECK: random cannot mint with jeff as owner
        let random = mock_info("random", &[]);
        let err = entry::execute(deps.as_mut(), mock_env(), random, mint_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // jeff cannot mint FOR random
        let allowed = mock_info(MINTER, &[]);
        let bad_mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: String::from("random"), // i.e. not jeff
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });
        let err2 =
            entry::execute(deps.as_mut(), mock_env(), allowed.clone(), bad_mint_msg).unwrap_err();
        assert_eq!(err2, ContractError::Unauthorized {});

        // jeff can mint
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // unknown nft returns error
        let _ = contract
            .nft_info(deps.as_ref(), "unknown".to_string())
            .unwrap_err();

        // CHECK: this nft info is correct
        let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: meta,
            }
        );

        // CHECK: owner info is correct
        let owner = contract
            .owner_of(deps.as_ref(), mock_env(), token_id.clone(), true)
            .unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: String::from("jeff-vader"),
                approvals: vec![],
            }
        );

        // CHECK: random can mint if sender && owner the same
        let john_q_rando = "random-guy";
        // another very plausible username imo
        let john_token_id = "johnisthebest".to_string();
        let john_token_uri = "https://example.com/jeff-vader".to_string();

        let john_q_rando_meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let john_q_rando_mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: john_token_id.clone(),
            owner: String::from(john_q_rando),
            token_uri: Some(john_token_uri.clone()),
            extension: john_q_rando_meta.clone(),
        });

        let not_jeff_minter = mock_info(john_q_rando, &[]);
        let _ = entry::execute(
            deps.as_mut(),
            mock_env(),
            not_jeff_minter,
            john_q_rando_mint_msg,
        )
        .unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(2, count.count);

        // CHECK: this nft info is correct
        let info = contract
            .nft_info(deps.as_ref(), john_token_id.clone())
            .unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(john_token_uri),
                extension: john_q_rando_meta,
            }
        );

        // CHECK: owner info is correct
        let owner = contract
            .owner_of(deps.as_ref(), mock_env(), john_token_id.clone(), true)
            .unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: String::from(john_q_rando),
                approvals: vec![],
            }
        );

        let meta2 = Metadata {
            twitter_id: Some(String::from("@jeff-vader-alt")),
            ..Metadata::default()
        };

        // CHECK: cannot mint same token_id again
        // even if minter & owner are the same
        let mint_msg2 = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: None,
            extension: meta2.clone(),
        });

        let allowed = mock_info("jeff-vader", &[]);
        let err = entry::execute(deps.as_mut(), mock_env(), allowed, mint_msg2).unwrap_err();
        assert_eq!(err, ContractError::Claimed {});

        // list the token_ids
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(2, tokens.tokens.len());
        assert_eq!(vec![token_id.clone(), john_token_id.clone()], tokens.tokens);

        // CHECK: can mint second NFT
        // clearly there is an arms race by username proxy here
        let token_id_2 = "jeffisbetterthanjohn".to_string();
        let mint_msg3 = ExecuteMsg::Mint(MintMsg {
            token_id: token_id_2.clone(),
            owner: String::from("jeff-vader"),
            token_uri: None,
            extension: meta2.clone(),
        });

        let allowed = mock_info(MINTER, &[]);
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, mint_msg3).unwrap();

        // list the token_ids
        // CHECK: four calls to mint, 3 tokens minted
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(3, tokens.tokens.len());
        assert_eq!(
            vec![token_id_2.clone(), token_id.clone(), john_token_id.clone()],
            tokens.tokens
        );

        // CHECK: cannot mint third NFT
        // as we set token cap at 2 earlier
        // jeff wants to create another alias
        let token_id_3 = "jeffisactuallybrian".to_string();
        let mint_msg4 = ExecuteMsg::Mint(MintMsg {
            token_id: token_id_3,
            owner: String::from("jeff-vader"),
            token_uri: None,
            extension: meta2,
        });

        let allowed = mock_info(MINTER, &[]);
        let token_cap_err =
            entry::execute(deps.as_mut(), mock_env(), allowed, mint_msg4).unwrap_err();
        assert_eq!(token_cap_err, ContractError::TokenCapExceeded {});

        // list the token_ids
        // CHECK: five calls to mint, 3 tokens minted
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(3, tokens.tokens.len());
        assert_eq!(
            vec![token_id_2.clone(), token_id.clone(), john_token_id],
            tokens.tokens
        );

        // CHECK: no paths minted
        // Jeff has 2 base tokens
        let paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Paths {
                    owner: String::from("jeff-vader"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            paths_query_res,
            TokensResponse {
                tokens: [].to_vec()
            }
        );

        let base_tokens_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BaseTokens {
                    owner: String::from("jeff-vader"),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            base_tokens_query_res,
            TokensResponse {
                tokens: [token_id_2, token_id].to_vec()
            }
        );
    }

    #[test]
    fn subdomain_minting() {
        let allowed = mock_info(MINTER, &[]);
        let mut deps = mock_dependencies();
        let jeff_address = String::from("jeff-vader");

        let contract = setup_contract(deps.as_mut());
        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(4),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5, // small enough that "jeff" will be caught
                surcharge_fee: Uint128::new(1_500_000),
            }),
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), allowed.clone(), init_msg).unwrap();

        // init a plausible username
        let token_id = "jeffvader".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // jeff can mint
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // unknown nft returns error
        let _ = contract
            .nft_info(deps.as_ref(), "unknown".to_string())
            .unwrap_err();

        // CHECK: this nft info is correct
        let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: meta.clone(),
            }
        );

        // CHECK: owner info is correct
        let owner = contract
            .owner_of(deps.as_ref(), mock_env(), token_id.clone(), true)
            .unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: String::from("jeff-vader"),
                approvals: vec![],
            }
        );

        // CHECK: with no parent
        let no_parent_token_id_res = entry::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetParentId {
                token_id: token_id.clone(),
            },
        )
        .unwrap_err();

        // parent token id should equal the first minted token_id
        assert_eq!(
            no_parent_token_id_res,
            StdError::NotFound {
                kind: "Parent not found".to_string()
            }
        );

        // this should fail for a random
        // but succeed for jeff
        let subdomain_meta = Metadata {
            parent_token_id: Some(token_id.clone()),
            ..Metadata::default()
        };

        let subdomain_id = String::from("subdomain");

        // CHECK: random cannot mint a subdomain with jeff as owner of parent
        let random_subdomain_mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: String::from("subdomain"),
            owner: String::from("random"),
            token_uri: Some(token_uri.clone()),
            extension: subdomain_meta.clone(),
        });
        let random = mock_info("random", &[]);
        let err = entry::execute(deps.as_mut(), mock_env(), random, random_subdomain_mint_msg)
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // CHECK: ensure num tokens does not increase
        let count2 = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count2.count);

        // CHECK: jeff can mint a subdomain
        let subdomain_uri = "https://example.com/jeffvader/subdomain".to_string();
        let mint_msg_2 = ExecuteMsg::Mint(MintMsg {
            token_id: subdomain_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(subdomain_uri.clone()),
            extension: subdomain_meta.clone(),
        });
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg_2).unwrap();

        // CHECK: ensure num tokens increases
        let count3 = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(2, count3.count);

        // CHECK: this nft info is correct
        let subdomain_info = contract
            .nft_info(deps.as_ref(), subdomain_id.clone())
            .unwrap();
        assert_eq!(
            subdomain_info,
            WhoamiNftInfoResponse {
                token_uri: Some(subdomain_uri),
                extension: subdomain_meta,
            }
        );

        // CHECK: owner info is correct
        let owner = contract
            .owner_of(deps.as_ref(), mock_env(), subdomain_id.clone(), true)
            .unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: jeff_address.clone(),
                approvals: vec![],
            }
        );

        // CHECK: address mapping is correct
        let address_query_res: AddressOfResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AddressOf {
                    token_id: subdomain_id.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            address_query_res,
            AddressOfResponse {
                owner: jeff_address,
                contract_address: None,
                validator_address: None,
            }
        );

        // CHECK: can get parent ID
        let parent_id_query_res: GetParentIdResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetParentId {
                    token_id: subdomain_id.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        // parent token id should equal the first minted token_id
        assert_eq!(parent_id_query_res.parent_token_id, token_id);

        // CHECK: can get parent NFT info
        let parent_id_query_res: WhoamiNftInfoResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetParentInfo {
                    token_id: subdomain_id.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            parent_id_query_res,
            WhoamiNftInfoResponse {
                token_uri: Some(token_uri.clone()),
                extension: meta,
            }
        );

        // CHECK: we need to go ~derper~ deeper
        // get path for subdomain
        let path_query_res: GetPathResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetFullPath {
                    token_id: subdomain_id.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            path_query_res,
            GetPathResponse {
                path: String::from("jeffvader/subdomain")
            }
        );

        // CHECK: jeff can mint a sub-subdomain
        let deeper_subdomain_meta = Metadata {
            parent_token_id: Some(subdomain_id),
            ..Metadata::default()
        };
        let subdomain2_id = String::from("deeper");
        let subdomain2_uri = "https://example.com/jeffvader/subdomain/deeper".to_string();

        let mint_msg_3 = ExecuteMsg::Mint(MintMsg {
            token_id: subdomain2_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(subdomain2_uri.clone()),
            extension: deeper_subdomain_meta.clone(),
        });
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg_3).unwrap();

        // CHECK: ensure num tokens increases
        let count4 = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(3, count4.count);

        // CHECK: this nft info is correct
        let subdomain_info = contract
            .nft_info(deps.as_ref(), subdomain2_id.clone())
            .unwrap();
        assert_eq!(
            subdomain_info,
            WhoamiNftInfoResponse {
                token_uri: Some(subdomain2_uri),
                extension: deeper_subdomain_meta,
            }
        );

        let deeper_path_query_res: GetPathResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetFullPath {
                    token_id: subdomain2_id.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            deeper_path_query_res,
            GetPathResponse {
                path: String::from("jeffvader/subdomain/deeper")
            }
        );

        // CHECK: mint a path
        let path_id = "secret-plans".to_string();

        let path_meta = Metadata {
            parent_token_id: Some(subdomain2_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: path_meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id = format!("{}::{}", subdomain2_id, path_id);
        assert_eq!(prepended_path_id, "deeper::secret-plans");

        // CHECK: this path info is correct
        let path_info = contract
            .nft_info(deps.as_ref(), prepended_path_id.clone())
            .unwrap();
        assert_eq!(
            path_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: path_meta,
            }
        );

        // CHECK: mint a second path
        let path_id_2 = "death-star-1".to_string();

        let path_meta_2 = Metadata {
            parent_token_id: Some(prepended_path_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg_2 = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id_2.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: path_meta_2.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, path_mint_msg_2).unwrap();

        let prepended_path_id_2 = format!("{}::{}", prepended_path_id, path_id_2);
        assert_eq!(prepended_path_id_2, "deeper::secret-plans::death-star-1");

        // CHECK: this path info is correct
        let path_info_2 = contract
            .nft_info(deps.as_ref(), prepended_path_id_2.clone())
            .unwrap();
        assert_eq!(
            path_info_2,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: path_meta_2,
            }
        );

        // CHECK: 1 direct subpaths under deeper::secret-plans minted
        let secret_plans_nested_token_query: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: String::from("jeff-vader"),
                    token_id: prepended_path_id,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["deeper::secret-plans::death-star-1"]
        assert_eq!(
            secret_plans_nested_token_query,
            TokensResponse {
                tokens: [prepended_path_id_2.clone()].to_vec()
            }
        );

        // CHECK: finally, check the whole path
        let secret_plans_path_query: GetPathResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetFullPath {
                    token_id: prepended_path_id_2,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            secret_plans_path_query,
            GetPathResponse {
                path: String::from("jeffvader/subdomain/deeper::secret-plans::death-star-1")
            }
        );
    }

    #[test]
    fn path_minting() {
        let allowed = mock_info(MINTER, &[]);
        let mut deps = mock_dependencies();
        let jeff_address = String::from("jeff-vader");

        let contract = setup_contract(deps.as_mut());
        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(6),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5, // small enough that "jeff" will be caught
                surcharge_fee: Uint128::new(1_500_000),
            }),
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), allowed.clone(), init_msg).unwrap();

        // init a plausible username
        let token_id = "jeffvader".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();

        let lordvader_id = "lordvader".to_string();

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // another top level token
        let meta_2 = Metadata {
            ..Metadata::default()
        };

        let mint_msg_2 = ExecuteMsg::Mint(MintMsg {
            token_id: lordvader_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta_2,
        });

        // jeff can mint
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg_2).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(2, count.count);

        // unknown nft returns error
        let _ = contract
            .nft_info(deps.as_ref(), "unknown".to_string())
            .unwrap_err();

        // CHECK: this nft info is correct
        let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: meta,
            }
        );

        // CHECK: owner info is correct
        let owner = contract
            .owner_of(deps.as_ref(), mock_env(), token_id.clone(), true)
            .unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: jeff_address.clone(),
                approvals: vec![],
            }
        );

        // CHECK: mint a path
        let path_id = "employment".to_string();

        let path_meta = Metadata {
            parent_token_id: Some(token_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: path_meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id = format!("{}::{}", token_id, path_id);

        // CHECK: this path info is correct
        let path_info = contract
            .nft_info(deps.as_ref(), prepended_path_id.clone())
            .unwrap();
        assert_eq!(
            path_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: path_meta,
            }
        );

        // CHECK: mint another path
        let path_id_2 = "death-star-1".to_string();

        let path_meta_2 = Metadata {
            parent_token_id: Some(prepended_path_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg_2 = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id_2.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: path_meta_2.clone(),
        });

        let _ =
            entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg_2).unwrap();

        let prepended_path_id_2 = format!("{}::{}", prepended_path_id, path_id_2);

        // CHECK: this path info is correct
        let path_2_info = contract
            .nft_info(deps.as_ref(), prepended_path_id_2.clone())
            .unwrap();
        assert_eq!(
            path_2_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: path_meta_2,
            }
        );

        // CHECK: mint another path
        let path_id_3 = "death-star-2".to_string();

        let path_meta_3 = Metadata {
            parent_token_id: Some(prepended_path_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg_3 = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id_3.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: path_meta_3.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, path_mint_msg_3).unwrap();

        let prepended_path_id_3 = format!("{}::{}", prepended_path_id, path_id_3);

        // CHECK: this path info is correct
        let path_3_info = contract
            .nft_info(deps.as_ref(), prepended_path_id_3.clone())
            .unwrap();
        assert_eq!(
            path_3_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: path_meta_3,
            }
        );

        // CHECK: 3 paths minted
        let paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Paths {
                    owner: jeff_address.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["jeffvader::employment::death-star-1", "jeffvader::employment::death-star-2"]
        assert_eq!(
            paths_query_res,
            TokensResponse {
                tokens: [
                    prepended_path_id.clone(),
                    prepended_path_id_2.clone(),
                    prepended_path_id_3.clone()
                ]
                .to_vec()
            }
        );

        // CHECK: 3 direct subpaths under base token minted
        let jeffvader_paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: token_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["jeffvader::employment", "jeffvader::employment::death-star-1", "jeffvader::employment::death-star-2"]
        assert_eq!(
            jeffvader_paths_query_res,
            TokensResponse {
                tokens: [
                    prepended_path_id,
                    prepended_path_id_2.clone(),
                    prepended_path_id_3
                ]
                .to_vec()
            }
        );

        // CHECK: 0 paths under second base token minted
        let lordvader_paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: lordvader_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be []
        assert_eq!(
            lordvader_paths_query_res,
            TokensResponse {
                tokens: [].to_vec()
            }
        );

        // CHECK: Jeff has 2 base tokens
        let base_tokens_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BaseTokens {
                    owner: jeff_address,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            base_tokens_query_res,
            TokensResponse {
                tokens: [token_id, lordvader_id].to_vec()
            }
        );

        // CHECK: final sense check of path
        let full_path_query: GetPathResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::GetFullPath {
                    token_id: prepended_path_id_2,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            full_path_query,
            GetPathResponse {
                path: String::from("jeffvader::employment::death-star-1")
            }
        );
    }

    #[test]
    fn base_minting_with_base_mint_fees_owed() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let jeff_address = "jeff-addr".to_string();

        let native_denom = "uatom".to_string();
        let expected_mint_fee = Uint128::new(1_000_000);

        let jeff_sender_info = mock_info(&jeff_address, &[]);
        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: native_denom.clone(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: Some(Uint128::new(1_000_000)),
            burn_percentage: Some(50),
            short_name_surcharge: None,
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            init_msg,
        )
        .unwrap();

        // init a plausible username
        let token_id = "jeffisthebest".to_string();

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta,
        };

        // CHECK: cannot mint with insufficient funds
        let failing_msg = ExecuteMsg::Mint(mint_msg.clone());
        let failing_mint_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                &jeff_address,
                &coins(Uint128::new(500_000).u128(), native_denom.clone()),
            ),
            failing_msg,
        )
        .unwrap_err();
        assert_eq!(failing_mint_res, ContractError::InsufficientFunds {});

        // CHECK: can successfully mint
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        let mint_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                &jeff_address,
                &coins(expected_mint_fee.u128(), native_denom.clone()),
            ),
            exec_msg,
        )
        .unwrap();

        let half_of_fee = expected_mint_fee * Decimal::percent(50);
        let msgs: Vec<CosmosMsg> = vec![
            BankMsg::Send {
                to_address: jeff_address,
                amount: coins(half_of_fee.u128(), native_denom.clone()),
            }
            .into(),
            BankMsg::Burn {
                amount: coins(half_of_fee.u128(), native_denom),
            }
            .into(),
        ];

        // should get a response with submsgs
        // todo - use multitest to simulate this better
        assert_eq!(
            mint_res,
            Response::new()
                .add_attribute("action", "mint")
                .add_attribute("minter", jeff_sender_info.sender)
                .add_attribute("token_id", token_id.clone())
                .add_messages(msgs)
        );

        let res = contract.nft_info(deps.as_ref(), token_id).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, mint_msg.extension);
    }

    #[test]
    fn base_minting_with_fees_and_surcharge_owed() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let jeff_address = "jeff-addr".to_string();

        let native_denom = "uatom".to_string();
        let base_mint_fee = Uint128::new(1_000_000);
        let expected_mint_fee = Uint128::new(2_000_000);

        let jeff_sender_info = mock_info(&jeff_address, &[]);
        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: native_denom.clone(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: Some(base_mint_fee),
            burn_percentage: None,
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5, // small enough that "jeff" will be caught
                surcharge_fee: Uint128::new(1_000_000),
            }),
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            init_msg,
        )
        .unwrap();

        // init a plausible username
        let token_id = "jeff".to_string();

        // CHECK: short username is caught
        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta.clone(),
        };

        // CHECK: cannot mint with insufficient funds
        // if they cover base but not surcharge
        let failing_msg = ExecuteMsg::Mint(mint_msg.clone());
        let failing_mint_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                &jeff_address,
                &coins(Uint128::new(1_500_000).u128(), native_denom.clone()),
            ),
            failing_msg,
        )
        .unwrap_err();
        assert_eq!(failing_mint_res, ContractError::InsufficientFunds {});

        // CHECK: can mint
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        let mint_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                &jeff_address,
                &coins(expected_mint_fee.u128(), native_denom.clone()),
            ),
            exec_msg,
        )
        .unwrap();

        // should get a response with submsgs
        // should cost 2_000_000
        // todo - use multitest to simulate this better
        // we expect this to be the expected_mint_fee
        let msgs: Vec<CosmosMsg> = vec![BankMsg::Send {
            to_address: jeff_address.to_string(),
            amount: coins(expected_mint_fee.u128(), native_denom.clone()),
        }
        .into()];

        assert_eq!(
            mint_res,
            Response::new()
                .add_attribute("action", "mint")
                .add_attribute("minter", &jeff_sender_info.sender)
                .add_attribute("token_id", token_id.clone())
                .add_messages(msgs)
        );

        let res = contract.nft_info(deps.as_ref(), token_id).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, mint_msg.extension);

        // CHECK: longer username is not caught
        let longer_id = "123456";
        let mint_msg2 = MintMsg {
            token_id: longer_id.to_string(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta,
        };
        let exec_msg2 = ExecuteMsg::Mint(mint_msg2);
        let mint_res2 = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                &jeff_address,
                &coins(base_mint_fee.u128(), native_denom.clone()),
            ),
            exec_msg2,
        )
        .unwrap();

        // we expect this to be the base_mint_fee
        let msgs2: Vec<CosmosMsg> = vec![BankMsg::Send {
            to_address: jeff_address,
            amount: coins(1_000_000, native_denom),
        }
        .into()];

        // todo - use multitest to simulate this better
        assert_eq!(
            mint_res2,
            Response::new()
                .add_attribute("action", "mint")
                .add_attribute("minter", jeff_sender_info.sender)
                .add_attribute("token_id", longer_id)
                .add_messages(msgs2)
        );
    }

    #[test]
    fn base_minting_with_fees_and_surcharge_owed_rounding_check() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let jeff_address = "jeff-addr".to_string();

        let native_denom = "uatom".to_string();
        let base_mint_fee = Uint128::new(1_250_333);
        // half of this number is 1_625_166.5 (i.e. smaller than decimals)
        // let expected_mint_fee = Uint128::new(3_250_333);

        let jeff_sender_info = mock_info(&jeff_address, &[]);
        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: native_denom.clone(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: Some(base_mint_fee),
            burn_percentage: Some(50),
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5, // small enough that "jeff" will be caught
                surcharge_fee: Uint128::new(2_000_000),
            }),
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            init_msg,
        )
        .unwrap();

        // init a plausible username
        let token_id = "jeff".to_string();

        // CHECK: short username is caught
        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta,
        };

        // CHECK: cannot mint with insufficient funds
        // if they cover base but not surcharge + burn
        // 2_000_000 covers all the mint fee
        // but not all the burn fee
        let failing_msg = ExecuteMsg::Mint(mint_msg.clone());
        let failing_mint_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                &jeff_address,
                &coins(Uint128::new(2_000_000).u128(), native_denom.clone()),
            ),
            failing_msg,
        )
        .unwrap_err();
        assert_eq!(failing_mint_res, ContractError::InsufficientFunds {});

        // CHECK: CAN MINT
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        let mint_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&jeff_address, &coins(3_250_333, native_denom.clone())),
            exec_msg,
        )
        .unwrap();

        // should get a response with submsgs
        // should cost 3_250_333
        // 1_625_166 sent and 1_625_166 burned
        // todo - use multitest to simulate this better
        let msgs: Vec<CosmosMsg> = vec![
            BankMsg::Send {
                to_address: jeff_address,
                amount: coins(1_625_166, native_denom.clone()),
            }
            .into(),
            BankMsg::Burn {
                amount: coins(1_625_166, native_denom),
            }
            .into(),
        ];

        assert_eq!(
            mint_res,
            Response::new()
                .add_attribute("action", "mint")
                .add_attribute("minter", &jeff_sender_info.sender)
                .add_attribute("token_id", token_id.clone())
                .add_messages(msgs)
        );

        let res = contract.nft_info(deps.as_ref(), token_id).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, mint_msg.extension);
    }

    #[test]
    fn base_minting_with_only_surcharge_owed() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let jeff_address = "jeff-addr".to_string();

        let native_denom = "uatom".to_string();
        let expected_mint_fee = Uint128::new(1_500_000);

        let jeff_sender_info = mock_info(&jeff_address, &[]);
        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: native_denom.clone(),
            native_decimals: 6,
            token_cap: Some(2),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5, // small enough that "jeff" will be caught
                surcharge_fee: Uint128::new(1_500_000),
            }),
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };
        entry::instantiate(
            deps.as_mut(),
            mock_env(),
            jeff_sender_info.clone(),
            init_msg,
        )
        .unwrap();

        // init a plausible username
        let token_id = "jeff".to_string();

        // CHECK: short username is caught
        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta,
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        let mint_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                &jeff_address,
                &coins(expected_mint_fee.u128(), native_denom.clone()),
            ),
            exec_msg,
        )
        .unwrap();

        let half_of_fee = expected_mint_fee * Decimal::percent(50);
        let msgs: Vec<CosmosMsg> = vec![
            BankMsg::Send {
                to_address: jeff_address,
                amount: coins(half_of_fee.u128(), native_denom.clone()),
            }
            .into(),
            BankMsg::Burn {
                amount: coins(half_of_fee.u128(), native_denom),
            }
            .into(),
        ];

        // should get a response with submsgs
        // should cost 1_500_000
        // todo - use multitest to simulate this better
        assert_eq!(
            mint_res,
            Response::new()
                .add_attribute("action", "mint")
                .add_attribute("minter", &jeff_sender_info.sender)
                .add_attribute("token_id", token_id.clone())
                .add_messages(msgs)
        );

        let res = contract.nft_info(deps.as_ref(), token_id).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, mint_msg.extension);
    }

    #[test]
    fn update_metadata() {
        let mut deps = mock_dependencies();
        let contract = setup_contract(deps.as_mut());

        // init a plausible username
        let token_id = "thebestguy".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();
        let jeff_address = String::from("jeff-vader");

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // CHECK: jeff can mint
        let allowed = mock_info(&jeff_address, &[]);
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        let bad_update_msg = ExecuteMsg::UpdateMetadata(UpdateMetadataMsg {
            token_id: token_id.clone(),
            metadata: Metadata {
                twitter_id: Some(String::from("@john_q_rando")),
                ..Metadata::default()
            },
        });

        // CHECK: random cannot update
        let john_q_rando = "random-guy";
        let not_allowed_to_update = mock_info(john_q_rando, &[]);
        let err = entry::execute(
            deps.as_mut(),
            mock_env(),
            not_allowed_to_update,
            bad_update_msg,
        )
        .unwrap_err();

        assert_eq!(err, ContractError::Unauthorized {});

        // CHECK: this nft info is correct
        let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: meta,
            }
        );

        // CHECK jeff can update
        let new_meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader-2")),
            parent_token_id: Some("this-token-id-should-be-ignored".to_string()),
            ..Metadata::default()
        };

        let expected_meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader-2")),
            parent_token_id: None, // i.e. as it was
            ..Metadata::default()
        };
        let update_msg = ExecuteMsg::UpdateMetadata(UpdateMetadataMsg {
            token_id: token_id.clone(),
            metadata: new_meta,
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, update_msg).unwrap();

        let info = contract.nft_info(deps.as_ref(), token_id).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: expected_meta,
            }
        );
    }

    #[test]
    fn update_metadata_does_not_clear_parent() {
        let mut deps = mock_dependencies();
        let contract = setup_contract(deps.as_mut());

        // init a plausible username
        let token_id = "jeffvader".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();
        let jeff_address = String::from("jeff-vader");

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // CHECK: jeff can mint
        let allowed = mock_info(&jeff_address, &[]);
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // CHECK: this nft info is correct
        let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: meta,
            }
        );

        // CHECK: mint a path
        let path_id = "vehicles".to_string();
        let path_uri = "https://example.com/jeff-vader/vehicles".to_string();

        let path_meta = Metadata {
            parent_token_id: Some(token_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(path_uri.clone()),
            extension: path_meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id = format!("{}::{}", token_id, path_id);
        assert_eq!(prepended_path_id, "jeffvader::vehicles");

        // CHECK: this path info is correct
        let path_info = contract
            .nft_info(deps.as_ref(), prepended_path_id.clone())
            .unwrap();
        assert_eq!(
            path_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(path_uri.clone()),
                extension: path_meta,
            }
        );

        // CHECK jeff can update path meta
        // but parent cannot be changed
        let new_meta = Metadata {
            parent_token_id: Some("this-token-id-should-be-ignored".to_string()),
            ..Metadata::default()
        };

        let expected_meta = Metadata {
            parent_token_id: Some(token_id), // i.e. as it was
            ..Metadata::default()
        };
        let update_msg = ExecuteMsg::UpdateMetadata(UpdateMetadataMsg {
            token_id: prepended_path_id.clone(),
            metadata: new_meta,
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, update_msg).unwrap();

        let info = contract.nft_info(deps.as_ref(), prepended_path_id).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(path_uri),
                extension: expected_meta,
            }
        );
    }

    #[test]
    fn alias_and_paths_cleared_on_send() {
        // init a plausible username
        let token_id = "thebestguy".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();
        let jeff_address = String::from("jeff-vader");

        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(6),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5, // small enough that "jeff" will be caught
                surcharge_fee: Uint128::new(1_500_000),
            }),
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };

        let allowed = mock_info(&jeff_address, &[]);
        let _ = entry::instantiate(deps.as_mut(), mock_env(), allowed, init_msg);

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let default_meta = Metadata {
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // CHECK: jeff can mint
        let allowed = mock_info(&jeff_address, &[]);
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // CHECK: check alias returns something
        let alias_query_res: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res.username, token_id);

        // CHECK: mint a path
        let path_id = "secret-plans".to_string();

        let path_meta = Metadata {
            parent_token_id: Some(token_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: path_meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id = format!("{}::{}", token_id, path_id);

        // CHECK: this path info is correct
        let path_info = contract
            .nft_info(deps.as_ref(), prepended_path_id.clone())
            .unwrap();
        assert_eq!(
            path_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: path_meta,
            }
        );

        // CHECK: 1 path under base token minted
        let jeffvader_paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: token_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["thebestguy::secret-plans"]
        assert_eq!(
            jeffvader_paths_query_res,
            TokensResponse {
                tokens: [prepended_path_id.clone()].to_vec()
            }
        );

        // CHECK: can mint second NFT
        let token_id_2 = "jeffisbetterthanjohn".to_string();
        let mint_msg2 = ExecuteMsg::Mint(MintMsg {
            token_id: token_id_2.clone(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta,
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg2).unwrap();

        // CHECK: ensure num tokens increases to 2
        let count_2 = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(3, count_2.count);

        // default will be that last in is returned
        // CHECK: jeff alias will default to token_id_2
        let alias_query_res_2: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_2.username, token_id_2);

        // set alias to NFT 1
        let _update_alias_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            allowed.clone(),
            ExecuteMsg::UpdatePrimaryAlias {
                token_id: token_id.clone(),
            },
        );

        // CHECK: alias updated to token_id
        let alias_query_res_3: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_3.username, token_id);

        // okay time to send NFT 1
        let other_contract_address = "other-contract-address";

        // jeff sends the token to another contract cos YOLO
        let send_msg = ExecuteMsg::SendNft {
            contract: other_contract_address.to_string(),
            token_id: token_id.clone(),
            msg: to_binary("yolo").unwrap(),
        };

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, send_msg);

        // CHECK: jeff-address should be default alias NFT 2
        let alias_query_res_4: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_4.username, token_id_2);

        // CHECK: this nft info META is correct
        // i.e. it has been reset to the default
        let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: default_meta,
            }
        );

        // CHECK: no path
        let path_info_after_send = contract
            .nft_info(deps.as_ref(), prepended_path_id)
            .unwrap_err();
        assert_eq!(
            path_info_after_send,
            StdError::NotFound {
                kind: "cw721_base::state::TokenInfo<whoami::msg::Metadata>".to_string()
            }
        );

        // CHECK: no path under base token
        let jeffvader_paths_query_res_after_send: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address,
                    token_id,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            jeffvader_paths_query_res_after_send,
            TokensResponse {
                tokens: [].to_vec()
            }
        );
    }

    #[test]
    fn can_transfer_path() {
        // init a plausible username
        let token_id = "star-wars".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();
        let jeff_address = String::from("jeff-vader");

        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(6),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5, // small enough that "jeff" will be caught
                surcharge_fee: Uint128::new(1_500_000),
            }),
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };

        let allowed = mock_info(&jeff_address, &[]);
        entry::instantiate(deps.as_mut(), mock_env(), allowed.clone(), init_msg).unwrap();

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let default_meta = Metadata {
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta,
        });

        // CHECK: jeff can mint
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // CHECK: alias returns something
        let alias_query_res: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res.username, token_id);

        // CHECK: mint a path
        let path_id = "vehicles".to_string();

        let path_meta = Metadata {
            parent_token_id: Some(token_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: path_meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id = format!("{}::{}", token_id, path_id);

        // CHECK: this path info is correct
        let path_info = contract
            .nft_info(deps.as_ref(), prepended_path_id.clone())
            .unwrap();
        assert_eq!(
            path_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: path_meta,
            }
        );

        // CHECK: 1 path under base token minted
        let jeffvader_paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: token_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["thebestguy::secret-plans"]
        assert_eq!(
            jeffvader_paths_query_res,
            TokensResponse {
                tokens: [prepended_path_id.clone()].to_vec()
            }
        );

        // CHECK: mint a second path
        let path_id_2 = "tie-fighter".to_string();

        let path_meta_2 = Metadata {
            parent_token_id: Some(prepended_path_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id_2.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: path_meta_2.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id_2 = format!("{}::{}", prepended_path_id, path_id_2);
        assert_eq!(prepended_path_id_2, "star-wars::vehicles::tie-fighter");

        // CHECK: this path info is correct
        let path_info_2 = contract
            .nft_info(deps.as_ref(), prepended_path_id_2.clone())
            .unwrap();
        assert_eq!(
            path_info_2,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: path_meta_2,
            }
        );

        // CHECK: 2 paths under base token minted
        let jeffvader_paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: token_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["star-wars::vehicles", "star-wars::vehicles::tie-fighter"]
        assert_eq!(
            jeffvader_paths_query_res,
            TokensResponse {
                tokens: [prepended_path_id.clone(), prepended_path_id_2.clone()].to_vec()
            }
        );

        // okay time to move path 1
        let john_q_rando_address = "random-guy";

        // he wants vehicles::tie-fighter so he buys the path off of jeff
        // and then jeff transfers the token
        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: john_q_rando_address.to_string(),
            token_id: prepended_path_id_2.clone(),
        };

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, transfer_msg);

        // CHECK: owner info is correct for prepended_path_id & prepended_path_id_2
        let path_1_owner = contract
            .owner_of(deps.as_ref(), mock_env(), prepended_path_id.clone(), true)
            .unwrap();
        assert_eq!(
            path_1_owner,
            OwnerOfResponse {
                owner: jeff_address.to_string(),
                approvals: vec![],
            }
        );
        let path_2_owner = contract
            .owner_of(deps.as_ref(), mock_env(), prepended_path_id_2.clone(), true)
            .unwrap();
        assert_eq!(
            path_2_owner,
            OwnerOfResponse {
                owner: john_q_rando_address.to_string(),
                approvals: vec![],
            }
        );

        // CHECK: john_q_rando_address alias is an error
        // as he only has a path but no base token
        let john_q_rando_primary_query = entry::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::PrimaryAlias {
                address: john_q_rando_address.to_string(),
            },
        )
        .unwrap_err();

        assert_eq!(
            john_q_rando_primary_query,
            StdError::NotFound {
                kind: "Primary alias not found".to_string()
            }
        );

        // CHECK: this path info META is correct
        // i.e. it has been reset to the default
        let info = contract
            .nft_info(deps.as_ref(), prepended_path_id_2)
            .unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: default_meta,
            }
        );

        // CHECK: only 1 path under base token
        let jeffvader_basetoken_paths_query_res_after_transfer: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            jeffvader_basetoken_paths_query_res_after_transfer,
            TokensResponse {
                tokens: [prepended_path_id.clone()].to_vec()
            }
        );

        // CHECK: jeff should only have the one path
        let jeffvader_all_paths_query_res_after_transfer: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Paths {
                    owner: jeff_address,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            jeffvader_all_paths_query_res_after_transfer,
            TokensResponse {
                tokens: [prepended_path_id].to_vec()
            }
        );
    }

    #[test]
    fn alias_and_paths_cleared_on_transfer() {
        // init a plausible username
        let token_id = "thebestguy".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();
        let jeff_address = String::from("jeff-vader");

        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(6),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5, // small enough that "jeff" will be caught
                surcharge_fee: Uint128::new(1_500_000),
            }),
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };

        let allowed = mock_info(&jeff_address, &[]);
        entry::instantiate(deps.as_mut(), mock_env(), allowed.clone(), init_msg).unwrap();

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let default_meta = Metadata {
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // CHECK: jeff can mint
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // CHECK: alias returns something
        let alias_query_res: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res.username, token_id);

        // CHECK: mint a path
        let path_id = "secret-plans".to_string();

        let path_meta = Metadata {
            parent_token_id: Some(token_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: path_meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id = format!("{}::{}", token_id, path_id);

        // CHECK: this path info is correct
        let path_info = contract
            .nft_info(deps.as_ref(), prepended_path_id.clone())
            .unwrap();
        assert_eq!(
            path_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: path_meta,
            }
        );

        // CHECK: 1 path under base token minted
        let jeffvader_paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: token_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["thebestguy::secret-plans"]
        assert_eq!(
            jeffvader_paths_query_res,
            TokensResponse {
                tokens: [prepended_path_id.clone()].to_vec()
            }
        );

        // CHECK: mint a second path
        let path_id_2 = "death-star-1".to_string();

        let path_meta_2 = Metadata {
            parent_token_id: Some(prepended_path_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id_2.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: path_meta_2.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id_2 = format!("{}::{}", prepended_path_id, path_id_2);
        assert_eq!(
            prepended_path_id_2,
            "thebestguy::secret-plans::death-star-1"
        );

        // CHECK: this path info is correct
        let path_info_2 = contract
            .nft_info(deps.as_ref(), prepended_path_id_2.clone())
            .unwrap();
        assert_eq!(
            path_info_2,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: path_meta_2,
            }
        );

        // CHECK: 2 paths under base token minted
        let jeffvader_paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: token_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["thebestguy::secret-plans"]
        assert_eq!(
            jeffvader_paths_query_res,
            TokensResponse {
                tokens: [prepended_path_id.clone(), prepended_path_id_2.clone()].to_vec()
            }
        );

        // CHECK: can mint second NFT
        let token_id_2 = "jeffisbetterthanjohn".to_string();
        let mint_msg2 = ExecuteMsg::Mint(MintMsg {
            token_id: token_id_2.clone(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta,
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg2).unwrap();

        // CHECK: ensure num tokens increases to 4 (i.e. 2 paths, 2 tokens)
        let count_2 = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(4, count_2.count);

        // default will be that last in is returned
        // CHECK: jeff alias will default to token_id_2
        let alias_query_res_2: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_2.username, token_id_2);

        // set alias to NFT 1
        let _update_alias_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            allowed.clone(),
            ExecuteMsg::UpdatePrimaryAlias {
                token_id: token_id.clone(),
            },
        );

        // CHECK: alias updated to token_id
        let alias_query_res_3: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_3.username, token_id);

        // okay time to move NFT 1
        let john_q_rando_address = "random-guy";

        // first he tries to pinch it
        let failed_john_transfer_msg = ExecuteMsg::TransferNft {
            recipient: john_q_rando_address.to_string(),
            token_id: token_id.clone(),
        };

        let not_allowed = mock_info(john_q_rando_address, &[]);

        let failed_transfer = entry::execute(
            deps.as_mut(),
            mock_env(),
            not_allowed,
            failed_john_transfer_msg,
        )
        .unwrap_err();
        assert_eq!(
            failed_transfer,
            ContractError::Base(cw721_base::ContractError::Unauthorized {})
        );

        // CHECK: despite the error, paths etc are untouched
        // first check alias
        let after_failure_query_res: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(after_failure_query_res.username, token_id);

        // CHECK: 2 paths STILL under base token minted
        let after_failure_paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: token_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["thebestguy::secret-plans", "thebestguy::secret-plans::death-star-1"]
        assert_eq!(
            after_failure_paths_query_res,
            TokensResponse {
                tokens: [prepended_path_id.clone(), prepended_path_id_2.clone()].to_vec()
            }
        );

        // he still really wants to be thebestguy so he buys the NFT off of jeff
        // and then jeff transfers the token
        let transfer_msg = ExecuteMsg::TransferNft {
            recipient: john_q_rando_address.to_string(),
            token_id: token_id.clone(),
        };

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, transfer_msg);

        // CHECK: owner info is correct
        let owner = contract
            .owner_of(deps.as_ref(), mock_env(), token_id.clone(), true)
            .unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: john_q_rando_address.to_string(),
                approvals: vec![],
            }
        );

        // CHECK: jeff-address should be default alias NFT 2 and john_q_rando_address
        // should be alias NFT 1
        // making him thebestguy
        let alias_query_res_4: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_4.username, token_id_2);

        let alias_query_res_5: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: john_q_rando_address.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_5.username, token_id);

        // CHECK: this nft info META is correct
        // i.e. it has been reset to the default
        let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: default_meta,
            }
        );

        // CHECK: no path
        let path_info_after_transfer = contract
            .nft_info(deps.as_ref(), prepended_path_id)
            .unwrap_err();
        assert_eq!(
            path_info_after_transfer,
            StdError::NotFound {
                kind: "cw721_base::state::TokenInfo<whoami::msg::Metadata>".to_string()
            }
        );

        // CHECK: no nested path (thebestguy::secret-plans::death-star-1) either
        let nested_path_info_after_transfer = contract
            .nft_info(deps.as_ref(), prepended_path_id_2)
            .unwrap_err();
        assert_eq!(
            nested_path_info_after_transfer,
            StdError::NotFound {
                kind: "cw721_base::state::TokenInfo<whoami::msg::Metadata>".to_string()
            }
        );

        // CHECK: no path under base token
        let jeffvader_basetoken_paths_query_res_after_transfer: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            jeffvader_basetoken_paths_query_res_after_transfer,
            TokensResponse {
                tokens: [].to_vec()
            }
        );

        // CHECK: jeff should actually have no paths left
        let jeffvader_all_paths_query_res_after_transfer: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Paths {
                    owner: jeff_address,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            jeffvader_all_paths_query_res_after_transfer,
            TokensResponse {
                tokens: [].to_vec()
            }
        );
    }

    #[test]
    fn alias_unchanged_if_primary_not_burned() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(3),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: None,
            admin_address: String::from(MINTER),
            username_length_cap: None,
        };
        let info = mock_info("creator", &[]);
        let res = entry::instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // init a plausible username
        let token_id = "jeffisthebest".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();
        let jeff_address = String::from("jeff-vader");

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri),
            extension: meta.clone(),
        });

        // CHECK: jeff can mint
        let allowed = mock_info(&jeff_address, &[]);
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // CHECK: alias returns something
        let alias_query_res: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res.username, token_id);

        // CHECK: can mint second NFT
        let token_id_2 = "jeffisbetterthanjohn".to_string();
        let mint_msg2 = ExecuteMsg::Mint(MintMsg {
            token_id: token_id_2.clone(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg2).unwrap();

        // CHECK: ensure num tokens increases to 2
        let count_2 = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(2, count_2.count);

        // CHECK: default will be that last in is returned
        let alias_query_res_2: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_2.username, token_id_2);

        // set alias to NFT 1
        let _update_alias_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            allowed.clone(),
            ExecuteMsg::UpdatePrimaryAlias {
                token_id: token_id.clone(),
            },
        );

        // CHECK: alias updated
        let alias_query_res_3: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_3.username, token_id);

        // CHECK: can mint third top level NFT
        let token_id_3 = "third-username".to_string();
        let mint_msg3 = ExecuteMsg::Mint(MintMsg {
            token_id: token_id_3,
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta,
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg3).unwrap();

        // CHECK: ensure num tokens increases to 3
        let count_3 = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(3, count_3.count);

        // CHECK: alias NOT updated
        let alias_query_res_4: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_4.username, token_id);

        // let's burn
        let burn_msg = ExecuteMsg::Burn {
            token_id: token_id_2,
        };

        // CHECK: random cannot burn
        let john_q_rando = "random-guy";

        let not_allowed_to_burn = mock_info(john_q_rando, &[]);
        let err = entry::execute(
            deps.as_mut(),
            mock_env(),
            not_allowed_to_burn,
            burn_msg.clone(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::Base(cw721_base::ContractError::Unauthorized {})
        );

        // then check jeff can
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, burn_msg).unwrap();

        // ensure num tokens decreases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(2, count.count);

        // CHECK: now preferred should return the unchanged token_id
        // that was set as primary
        let alias_query_res_5: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_5.username, token_id);
    }

    #[test]
    fn alias_and_paths_cleared_on_burn() {
        // init a plausible username
        let token_id = "thebestguy".to_string();
        let token_uri = "https://example.com/jeff-vader".to_string();
        let jeff_address = String::from("jeff-vader");

        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let init_msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: Some(6),
            base_mint_fee: None,
            burn_percentage: Some(50),
            short_name_surcharge: Some(SurchargeInfo {
                surcharge_max_characters: 5, // small enough that "jeff" will be caught
                surcharge_fee: Uint128::new(1_500_000),
            }),
            admin_address: jeff_address.clone(),
            username_length_cap: None,
        };

        let allowed = mock_info(&jeff_address, &[]);
        entry::instantiate(deps.as_mut(), mock_env(), allowed, init_msg).unwrap();

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg {
            token_id: token_id.clone(),
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // CHECK: jeff can mint
        let allowed = mock_info(&jeff_address, &[]);
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg).unwrap();

        // CHECK: ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // CHECK: alias returns something
        let alias_query_res: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res.username, token_id);

        // CHECK: mint a path
        let path_id = "secret-plans".to_string();

        let path_meta = Metadata {
            parent_token_id: Some(token_id.clone()),
            ..Metadata::default()
        };

        let path_mint_msg = ExecuteMsg::MintPath(MintMsg {
            token_id: path_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: path_meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), path_mint_msg).unwrap();

        let prepended_path_id = format!("{}::{}", token_id, path_id);

        // CHECK: this path info is correct
        let path_info = contract
            .nft_info(deps.as_ref(), prepended_path_id.clone())
            .unwrap();
        assert_eq!(
            path_info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri.clone()),
                extension: path_meta,
            }
        );

        // CHECK: 1 path under base token minted
        let jeffvader_paths_query_res: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: token_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["thebestguy::secret-plans"]
        assert_eq!(
            jeffvader_paths_query_res,
            TokensResponse {
                tokens: [prepended_path_id.clone()].to_vec()
            }
        );

        // CHECK: can mint second NFT
        let token_id_2 = "jeffisbetterthanjohn".to_string();
        let mint_msg2 = ExecuteMsg::Mint(MintMsg {
            token_id: token_id_2.clone(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: meta.clone(),
        });

        let _ = entry::execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg2).unwrap();

        // CHECK: ensure num tokens increases to 3
        let count_2 = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(3, count_2.count);

        // CHECK: default will be that last in is returned
        let alias_query_res_2: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_2.username, token_id_2);

        // set alias to NFT 1
        let _update_alias_res = entry::execute(
            deps.as_mut(),
            mock_env(),
            allowed.clone(),
            ExecuteMsg::UpdatePrimaryAlias {
                token_id: token_id.clone(),
            },
        );

        // CHECK: alias updated
        let alias_query_res_3: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_3.username, token_id);

        // let's burn
        let burn_msg = ExecuteMsg::Burn {
            token_id: token_id.clone(),
        };

        // CHECK: random cannot burn
        let john_q_rando = "random-guy";

        let not_allowed_to_burn = mock_info(john_q_rando, &[]);
        let err = entry::execute(
            deps.as_mut(),
            mock_env(),
            not_allowed_to_burn,
            burn_msg.clone(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            ContractError::Base(cw721_base::ContractError::Unauthorized {})
        );

        // CHECK: still 1 path under base token minted
        let jeffvader_paths_query_res_after_failed_burn: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address.clone(),
                    token_id: token_id.clone(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // expect response to be ["thebestguy::secret-plans"]
        assert_eq!(
            jeffvader_paths_query_res_after_failed_burn,
            TokensResponse {
                tokens: [prepended_path_id.clone()].to_vec()
            }
        );

        // CHECK: NFT info intact after failed burn
        let nft_info_john_tried_to_burn =
            contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            nft_info_john_tried_to_burn,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: meta,
            }
        );

        // then check jeff can burn
        let _ = entry::execute(deps.as_mut(), mock_env(), allowed, burn_msg).unwrap();

        // ensure num tokens decreases
        // it decreases by 2 as we are burning the token and the path
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // CHECK: now preferred should return default
        let alias_query_res_4: PrimaryAliasResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PrimaryAlias {
                    address: jeff_address.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(alias_query_res_4.username, token_id_2);

        // CHECK: no path
        let path_info_after_burn = contract
            .nft_info(deps.as_ref(), prepended_path_id)
            .unwrap_err();
        assert_eq!(
            path_info_after_burn,
            StdError::NotFound {
                kind: "cw721_base::state::TokenInfo<whoami::msg::Metadata>".to_string()
            }
        );

        // CHECK: no path under base token
        let jeffvader_paths_query_res_after_burn: TokensResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::PathsForToken {
                    owner: jeff_address,
                    token_id,
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            jeffvader_paths_query_res_after_burn,
            TokensResponse {
                tokens: [].to_vec()
            }
        );
    }

    #[test]
    fn use_metadata_extension() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: None,
            base_mint_fee: None,
            burn_percentage: None,
            short_name_surcharge: None,
            admin_address: "jeff-addr".to_string(),
            username_length_cap: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // mock info contains sender &&
        // info.sender and owner need to be the same
        // that & MINTER do not need to be
        // as MINTER is the admin addr on the contract
        let token_id = "enterprise";
        let contract_address = "contract-address".to_string();
        let validator_address = "validator_address".to_string();
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: CREATOR.to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Metadata {
                twitter_id: Some(String::from("@jeff-vader")),
                contract_address: Some(contract_address.clone()),
                validator_operator_address: Some(validator_address.clone()),
                ..Metadata::default()
            },
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, mint_msg.extension);

        let contract_query_res: IsContractResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsContract {
                    token_id: token_id.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(contract_query_res.contract_address, contract_address);

        // CHECK: address_of returns the right thing
        let address_of_res: AddressOfResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AddressOf {
                    token_id: token_id.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            address_of_res,
            AddressOfResponse {
                owner: CREATOR.to_string(),
                contract_address: Some(contract_address),
                validator_address: Some(validator_address)
            }
        );
    }

    #[test]
    fn is_contract_default_path_errors() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: None,
            base_mint_fee: None,
            burn_percentage: None,
            short_name_surcharge: None,
            admin_address: "jeff-addr".to_string(),
            username_length_cap: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // mock info contains sender &&
        // info.sender and owner need to be the same
        // that & MINTER do not need to be
        // as MINTER is the admin addr on the contract
        let token_id = "enterprise";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: CREATOR.to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Metadata {
                twitter_id: Some(String::from("@jeff-vader")),
                ..Metadata::default()
            },
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, mint_msg.extension);

        let contract_query_res = entry::query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::IsContract {
                token_id: token_id.to_string(),
            },
        )
        .unwrap_err();

        assert_eq!(
            contract_query_res,
            StdError::NotFound {
                kind: "No contract address".to_string()
            }
        );

        // CHECK: address_of returns the right thing
        let address_of_res: AddressOfResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AddressOf {
                    token_id: token_id.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(address_of_res.contract_address, None);
    }

    #[test]
    fn name_refers_to_contract() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            native_denom: "uatom".to_string(),
            native_decimals: 6,
            token_cap: None,
            base_mint_fee: None,
            burn_percentage: None,
            short_name_surcharge: None,
            admin_address: "jeff-addr".to_string(),
            username_length_cap: None,
        };
        entry::instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        // let's imagine this is a contract that does something
        // to do with the enterprise
        let contract_address = "contract-address".to_string();
        let token_id = "enterprise-contract";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: CREATOR.to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Metadata {
                contract_address: Some(contract_address.clone()),
                ..Metadata::default()
            },
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.extension, mint_msg.extension);

        let contract_query_res: IsContractResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsContract {
                    token_id: token_id.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(contract_query_res.contract_address, contract_address);

        // CHECK: address_of returns the right thing
        let address_of_res: AddressOfResponse = from_binary(
            &entry::query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AddressOf {
                    token_id: token_id.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            address_of_res,
            AddressOfResponse {
                owner: CREATOR.to_string(),
                contract_address: Some(contract_address),
                validator_address: None
            }
        );
    }
}
