#!/usr/bin/env bash
# Deterministic SBF build of the provider. Prints the sha256 that DEVNET.md /
# evidence/ record. Same inputs => same bytes (checked: two clean target dirs
# produce identical hashes).
#
# Pinned inputs:
#   - platform-tools v1.58 (rustc 1.95.0-dev inside platform-tools), --arch v3
#   - cargo-build-sbf from Agave CLI 3.1.10 (any CLI that accepts --tools-version v1.58)
#   - Cargo.lock (programs/musebook-credit-provider/Cargo.lock), built --locked
#   - vendor/payment-channels submodule at 23193ab
# Usage: scripts/build-verifiable.sh [default|devnet]
set -euo pipefail
cd "$(dirname "$0")/.."
flavor="${1:-default}"
features=()
out=target/verifiable/$flavor
[[ "$flavor" == devnet ]] && features=(--features devnet-deployment)
test "$(git -C vendor/payment-channels rev-parse --short=7 HEAD)" = 23193ab
rm -rf "target/verifiable-build-$flavor" "$out"
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$PWD/target/verifiable-build-$flavor" \
  cargo build-sbf --tools-version v1.58 --arch v3 \
  --manifest-path programs/musebook-credit-provider/Cargo.toml \
  --sbf-out-dir "$out" "${features[@]}" -- --locked
sha256sum "$out/musebook_credit_provider.so"
