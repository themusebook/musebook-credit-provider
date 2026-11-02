#!/usr/bin/env bash
# DEVNET ONLY. Deploys the devnet-deployment build and initializes governance.
# Refuses any other cluster. Keypairs are read from paths you pass; this script
# never prints them.
#   PAYER=path/to/devnet-payer.json PROGRAM_KEYPAIR=path/to/program.json \
#   ADMIN=<pubkey, e.g. Squads v4 vault PDA> scripts/devnet-deploy.sh
set -euo pipefail
cd "$(dirname "$0")/.."
URL=https://api.devnet.solana.com
: "${PAYER:?}" "${PROGRAM_KEYPAIR:?}"
genesis=$(solana genesis-hash -u "$URL")
[[ "$genesis" == EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG ]] || { echo "not devnet"; exit 1; }
scripts/build-verifiable.sh devnet
so=target/verifiable/devnet/musebook_credit_provider.so
solana program deploy -u "$URL" --keypair "$PAYER" --upgrade-authority "$PAYER" \
  --program-id "$PROGRAM_KEYPAIR" "$so" --commitment confirmed
PROGRAM=$(solana-keygen pubkey "$PROGRAM_KEYPAIR")
solana program show -u "$URL" "$PROGRAM"
solana program dump -u "$URL" "$PROGRAM" /tmp/musebook-devnet-dump.so
echo "on-chain sha256 (dump is the ELF; compare to build above):"
sha256sum /tmp/musebook-devnet-dump.so
# Governance config init + pause/unpause smoke run via the e2e binary in
# devnet mode is not wired yet; see docs/DEVNET.md "Remaining".
