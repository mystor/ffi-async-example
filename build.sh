#!/bin/bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")"

# build it
cargo build --manifest-path rust-lib/Cargo.toml
( cd swift && swift build )
gradle -p kotlin build

# run it
echo
echo "swift/.build/debug/swift"
swift/.build/debug/swift

echo
echo "gradle -p kotlin run"
gradle -p kotlin run
