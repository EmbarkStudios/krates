#!/bin/bash
set -eux

base="$(dirname "$(realpath "$0")")"

function cm() {
    cargo +nightly metadata --format-version=1 "$@"
}

pushd "$base"

cm --all-features > ../all-features2.json
cm --manifest-path a/Cargo.toml --no-default-features > ../a2.json
cm --manifest-path Cargo.toml --all-features > ../top2.json

popd
