#!/bin/bash
set -e

cargo metadata --format-version=1 --manifest-path tests/pid/Cargo.toml > tests/pid-opaque.json
cargo +nightly metadata --format-version=1 --manifest-path tests/pid/Cargo.toml > tests/pid-stable.json
