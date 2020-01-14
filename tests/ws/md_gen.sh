#!/bin/bash
set -eux

base="$(dirname "$(realpath "$0")")"

function cm() {
    cargo +nightly metadata --format-version=1 "$@"
}

pushd "$base"

cm --all-features > ../all-features.json
cm --manifest-path a/Cargo.toml --no-default-features > ../a.json
cm --manifest-path b/Cargo.toml --no-default-features > ../b.json
cm --manifest-path c/Cargo.toml --no-default-features --features leftier-strings > ../c.json
cm --manifest-path c/Cargo.toml --no-default-features > ../c-no-defaults.json

popd
