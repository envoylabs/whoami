#!/bin/bash

if [ "$1" = "" ]
then
  echo "Usage: $0 1 arg required - juno address"
  exit
fi

CONTAINER_NAME="juno_whoami"
BINARY="docker exec -i $CONTAINER_NAME junod"
DENOM='ujunox'
CHAIN_ID='testing'
RPC='http://localhost:26657/'
TXFLAG="--gas-prices 0.1$DENOM --gas auto --gas-adjustment 1.3 -y -b block --chain-id $CHAIN_ID --node $RPC"
BLOCK_GAS_LIMIT=${GAS_LIMIT:-100000000} # should mirror mainnet

# compile
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.5

# copy wasm to docker container
docker cp artifacts/whoami.wasm $CONTAINER_NAME:/whoami.wasm

# validator addr
VALIDATOR_ADDR=$($BINARY keys show validator --address)
echo "Validator address:"
echo $VALIDATOR_ADDR

BALANCE_1=$($BINARY q bank balances $VALIDATOR_ADDR)
echo "Pre-store balance:"
echo $BALANCE_1

# you ideally want to run locally, get a user and then
# pass that addr in here
echo "Address to deploy contracts: $1"
echo "TX Flags: $TXFLAG"

# upload whoami wasm
# CONTRACT_RES=$($BINARY tx wasm store "/whoami.wasm" --from validator $TXFLAG --output json)
# echo $CONTRACT_RES
CONTRACT_CODE=$($BINARY tx wasm store "/whoami.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')
echo "Stored: $CONTRACT_CODE"

BALANCE_2=$($BINARY q bank balances $VALIDATOR_ADDR)
echo "Post-store balance:"
echo $BALANCE_2

BASE_MINT_FEE=1000000
SURCHARGE_FEE=1000000
TOTAL_FEE=$((BASE_MINT_FEE + SURCHARGE_FEE))

# instantiate the CW721
WHOAMI_INIT='{
  "admin_address": "'"$1"'",
  "name": "Decentralized Name Service",
  "symbol": "WHO",
  "native_denom": "'"$DENOM"'",
  "native_decimals": 6,
  "token_cap": null,
  "base_mint_fee": "'"$BASE_MINT_FEE"'",
  "burn_percentage": 50,
  "short_name_surcharge": {
    "surcharge_max_characters": 5,
    "surcharge_fee": "'"$SURCHARGE_FEE"'"
  },
  "username_length_cap": 20
}'
echo "$WHOAMI_INIT" | jq .
$BINARY tx wasm instantiate $CONTRACT_CODE "$WHOAMI_INIT" --from "validator" --label "whoami NFT nameservice" $TXFLAG --no-admin
RES=$?

# get contract addr
CONTRACT_ADDRESS=$($BINARY q wasm list-contract-by-code $CONTRACT_CODE --output json | jq -r '.contracts[-1]')

# provision juno default user
echo "clip hire initial neck maid actor venue client foam budget lock catalog sweet steak waste crater broccoli pipe steak sister coyote moment obvious choose" | $BINARY keys add test-user --recover --keyring-backend test

# init name
MINT='{
  "mint": {
    "owner": "'"$1"'",
    "token_id": "nigeltufnel",
    "token_uri": null,
    "extension": {
      "image": null,
      "image_data": null,
      "email": null,
      "external_url": null,
      "public_name": "Nigel Tufnel",
      "public_bio": "Nigel Tufnel was born in Squatney, East London on February 5, 1948. He was given his first guitar, a Sunburst Rhythm King, by his father at age six. His life changed when he met David St. Hubbins who lived next door. They began jamming together in a toolshed in his garden, influenced by early blues artists like Honkin Bubba Fulton, Little Sassy Francis and particularly Big Little Daddy Coleman, a deaf guitar player.",
      "twitter_id": null,
      "discord_id": null,
      "telegram_id": null,
      "keybase_id": null,
      "validator_operator_address": ""
    }
  }
}'

$BINARY tx wasm execute "$CONTRACT_ADDRESS" "$MINT" --from test-user $TXFLAG --amount 1000000ujunox


# Print out config variables
printf "\n ------------------------ \n"
printf "Config Variables \n\n"

echo "NEXT_PUBLIC_WHOAMI_CODE_ID=$CONTRACT_CODE"
echo "NEXT_PUBLIC_WHOAMI_ADDRESS=$CONTRACT_ADDRESS"
echo "NEXT_PUBLIC_BASE_MINT_FEE=$BASE_MINT_FEE"
echo "NEXT_PUBLIC_SURCHARGE_FEE=$SURCHARGE_FEE"

echo $RES
exit $RES
