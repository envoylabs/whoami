#!/bin/bash

CONTAINER_NAME="juno_node_1"
BINARY="docker exec -i $CONTAINER_NAME junod"
DENOM='ujunox'
CHAIN_ID='testing'
RPC='http://localhost:26657/'
TXFLAG="--gas-prices 0.1$DENOM --gas auto --gas-adjustment 1.3 -y -b block --chain-id $CHAIN_ID --node $RPC"
BLOCK_GAS_LIMIT=${GAS_LIMIT:-100000000} # should mirror mainnet

# start a juno container
docker stop juno_node_1 && docker rm juno_node_1 && docker volume rm -f junod_data
docker run --rm -it --name juno_node_1 \
    -e PASSWORD=xxxxxxxxx \
    -e STAKE_TOKEN=ujunox \
    -e GAS_LIMIT="100000000" \
    -e UNSAFE_CORS=true \
    -p 1317:1317 -p 26656:26656 -p 26657:26657 \
    --mount type=volume,source=junod_data,target=/root \
    ghcr.io/cosmoscontracts/juno:v2.0.6 /bin/sh

# now inside the container we run
./setup_junod.sh juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y
./run_junod.sh > /dev/null 2>&1 &
junod status

# in a new tab
# download smart contract repo
# and get correct test script
git clone git@github.com:envoylabs/whoami.git
cd whoami
git checkout 20_byte_testing
./scripts/deploy_contracts.sh juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y

# at this point you should see a 20 byte address, juno14hj2tavq8fpesdwxxcu44rty3hh90vhuwxjqxx
# go back to the container tab
apk add --no-cache ca-certificates build-base git make bash gcc musl-dev openssl go

# build go
wget -O go.tgz https://go.dev/dl/go1.18.linux-amd64.tar.gz 
tar -C /usr/local -xzf go.tgz
cd /usr/local/go/src/ 
./make.bash 
export PATH="/usr/local/go/bin:$PATH"
export GOPATH=/opt/go/ 
export PATH=$PATH:$GOPATH/bin 
go version

# get new juno ver
mkdir /code
cd /code
git clone https://github.com/CosmosContracts/juno
cd juno
git checkout v2.3.0-beta.2

# find junod PID
ps aux | grep junod 

# kill it
kill <PID>

# build new version
wget -O /lib/libwasmvm_muslc.a https://github.com/CosmWasm/wasmvm/releases/download/v1.0.0-beta7/libwasmvm_muslc.a 
LEDGER_ENABLED=false BUILD_TAGS=muslc make build
cp /code/juno/bin/junod /usr/bin/junod
junod version

# junod version should now be v2.3.0-beta.2
# restart junod
junod start --rpc.laddr tcp://0.0.0.0:26657 --trace

# in the other window, try the party bus again
# second arg is whatever the logged value of NEXT_PUBLIC_WHOAMI_CODE_ID was
# probably 1
./scripts/execute_contract.sh juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y 1

# we instantiate again - this will be code 1
./scripts/instantiate_again.sh juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y 1
# and contract address will be [-1] so execute contract will
# run against this new one we just instantiated
# you can tell this as we are using the same token name and owner
# yet it should work
./scripts/execute_contract.sh juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y 1

# if it says aborted, the test user was probably already imported. don't worry about it

# finally, try another contract
./scripts/deploy_v230_contracts.sh juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y