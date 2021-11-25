#!/bin/bash

set -e

cargo clippy --all-targets -- -D warnings
RUST_BACKTRACE=full cargo unit-test
cargo schema
