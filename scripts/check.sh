#!/bin/bash

set -e

cargo clippy --all-targets -- -D warnings
RUST_BACKTRACE=full cargo unit-test
cargo fmt
START_DIR=$(pwd)
for f in ./contracts/*
do
  cd "$f"
  CMD="cargo run --example schema"
  eval $CMD
  cd "$START_DIR"
done

