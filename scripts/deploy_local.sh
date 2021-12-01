#!/bin/bash

# pinched and adapted from DA0DA0
# this rather assumes you're using juno bootstrap script
# this script takes an address to use inside the container
# you get this address when running the juno bootstrap - it will be logged
CONTAINER_NAME="juno_whoami"
BINARY="docker exec -i $CONTAINER_NAME junod"
DENOM='ustake'
CHAIN_ID='testing'
RPC='http://localhost:26657/'
TXFLAG="--gas-prices 0.01$DENOM --gas auto --gas-adjustment 1.3 -y -b block --chain-id $CHAIN_ID --node $RPC"

# run container
docker kill $CONTAINER_NAME
docker run --rm -d --name $CONTAINER_NAME -p 1317:1317 -p 26656:26656 -p 26657:26657 ghcr.io/cosmoscontracts/juno:pr-105 ./setup_and_run.sh $1

# compile
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.3

# copy wasm to docker container
docker cp artifacts/whoami.wasm $CONTAINER_NAME:/whoami.wasm

# you ideally want to run locally, get a user and then
# pass that addr in here
echo "Address to deploy contracts: $1"
echo "TX Flags: $TXFLAG"

# upload whoami wasm
CONTRACT_CODE=$($BINARY tx wasm store "/whoami.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')

echo "Stored: $CONTRACT_CODE"

# instantiate the CW721
WHOAMI_INIT='{
  "minter": "'"$1"'",
  "name": "Whoami Juno Name Service",
  "symbol": "WHO"
}'
echo "$WHOAMI_INIT"
$BINARY tx wasm instantiate $CONTRACT_CODE "$WHOAMI_INIT" --from "validator" --label "whoami NFT nameservice" $TXFLAG

# get contract addr
CONTRACT_ADDRESS=$($BINARY q wasm list-contract-by-code $CONTRACT_CODE --output json | jq -r '.contracts[-1]')

# Print out config variables
printf "\n ------------------------ \n"
printf "Config Variables \n\n"

echo "WHOAMI_CODE_ID=$CONTRACT_CODE"
echo "WHOAMI_ADDRESS=$CONTRACT_ADDRESS"
