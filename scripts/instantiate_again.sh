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
$BINARY tx wasm instantiate $2 "$WHOAMI_INIT" --from "validator" --label "whoami NFT nameservice" $TXFLAG --no-admin
RES=$?

# get contract addr
CONTRACT_ADDRESS=$($BINARY q wasm list-contract-by-code $2 --output json | jq -r '.contracts[-1]')

# Print out config variables
printf "\n ------------------------ \n"
printf "Config Variables \n\n"

echo "NEXT_PUBLIC_WHOAMI_CODE_ID=$2"
echo "NEXT_PUBLIC_WHOAMI_ADDRESS=$CONTRACT_ADDRESS"
echo "NEXT_PUBLIC_BASE_MINT_FEE=$BASE_MINT_FEE"
echo "NEXT_PUBLIC_SURCHARGE_FEE=$SURCHARGE_FEE"

echo $RES
exit $RES
