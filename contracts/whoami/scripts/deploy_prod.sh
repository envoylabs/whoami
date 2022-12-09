#!/bin/bash

# takes several arguments:
# 1: address to use for instantiating
# 2: key to use as --from argument
# 3: name of the contract
# 4: symbol for the contract
BINARY='junod'
DENOM='ujuno'
CHAIN_ID='juno-1'
RPC='http://localhost:26657/'
LABEL="Whoami NFT nameservice"
TXFLAG="--gas-prices auto --gas auto --gas-adjustment 2 -y -b block --chain-id $CHAIN_ID --node $RPC"

# compile
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.6

# presumably you know the addr you want to use already
echo "Address to deploy contracts: $1"
echo "TX Flags: $TXFLAG"

# upload whoami wasm
CONTRACT_CODE=$($BINARY tx wasm store "artifacts/whoami.wasm" --from $2 $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')

echo "Stored: $CONTRACT_CODE"

# instantiate the CW721
WHOAMI_INIT='{
  "admin_address": "'"$1"'",
  "name": "Decentralized Name Service",
  "symbol": "WHO",
  "native_denom": "'"$DENOM"'",
  "native_decimals": 6,
  "token_cap": null,
  "base_mint_fee": "1000000",
  "burn_percentage": 50,
  "short_name_surcharge": {
    "surcharge_max_characters": 5,
    "surcharge_fee": "1000000"
  }
}'
echo "$WHOAMI_INIT" | jq .
$BINARY tx wasm instantiate $CONTRACT_CODE "$WHOAMI_INIT" --from "$2" --label $LABEL $TXFLAG

# get contract addr
CONTRACT_ADDRESS=$($BINARY q wasm list-contract-by-code $CONTRACT_CODE --output json | jq -r '.contracts[-1]')

# Print out config variables
printf "\n ------------------------ \n"
printf "Config Variables \n\n"

echo "WHOAMI_CODE_ID=$CONTRACT_CODE"
echo "WHOAMI_ADDRESS=$CONTRACT_ADDRESS"
