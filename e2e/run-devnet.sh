#!/usr/bin/env bash
# DEVNET ONLY end-to-end run against the deployed provider and the devnet V2
# channel program (CHNLxBt…). Funds test accounts from the deployer key, never
# from a faucet. Prints signatures, never keys.
#   CREDIT_E2E_PAYER=path/to/deployer.json CREDIT_E2E_PROGRAM=<id> e2e/run-devnet.sh
set -euo pipefail
cd "$(dirname "$0")/.."
: "${CREDIT_E2E_PAYER:?}" "${CREDIT_E2E_PROGRAM:?}"
export CREDIT_E2E_RPC=https://api.devnet.solana.com
genesis=$(curl -s "$CREDIT_E2E_RPC" -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getGenesisHash"}' | sed 's/.*"result":"\([^"]*\)".*/\1/')
[[ "$genesis" == EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG ]] || { echo "not devnet"; exit 1; }
cargo run --manifest-path e2e/Cargo.toml --features devnet
