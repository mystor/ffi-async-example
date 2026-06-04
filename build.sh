#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
rust_dir="$root_dir/rust-lib"
build_dir="$root_dir/build"
profile="${PROFILE:-debug}"

cargo_args=(build --manifest-path "$rust_dir/Cargo.toml")
target_dir="$rust_dir/target/$profile"

if [[ "$profile" == "release" ]]; then
    cargo_args+=(--release)
fi

cargo "${cargo_args[@]}"

mkdir -p "$build_dir"

c++ \
    -std=c++20 \
    -pthread \
    "$root_dir/main.cpp" \
    "$target_dir/librust_lib.a" \
    -o "$build_dir/ffi-async-example"

echo "$build_dir/ffi-async-example"
