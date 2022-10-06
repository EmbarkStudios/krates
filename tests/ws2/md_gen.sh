#!/bin/bash
set -eux

base="$(dirname "$(realpath "$0")")"

function cm() {
    cargo +nightly metadata --format-version=1 "$@"
}

pushd "$base"

cm --all-features > ../all-features2.json

popd
