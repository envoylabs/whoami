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

# get contract addr
CONTRACT_ADDRESS=$($BINARY q wasm list-contract-by-code $2 --output json | jq -r '.contracts[-1]')

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

OUTPUT=$($BINARY tx wasm execute "$CONTRACT_ADDRESS" "$MINT" --from test-user $TXFLAG --amount 1000000ujunox --output json)
RES=$?

echo $OUTPUT | jq .
echo $RES
exit $RES
