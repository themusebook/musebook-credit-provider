# musebook-credit-provider

API credit provider for [musebook.trade](https://musebook.trade), built for Pay's
credit-provider model (preFund / postDistribute hooks on payment-channel V2).

> **Status: pre-audit. Deployed on Solana devnet only** as
> [`97F68uhs5fmvddJxH1LrKytbBXrhx6ZBZwuZAdUdP7QA`](https://explorer.solana.com/address/97F68uhs5fmvddJxH1LrKytbBXrhx6ZBZwuZAdUdP7QA?cluster=devnet)
> (see [docs/DEVNET.md](docs/DEVNET.md)). Upgrade authority and config admin are the
> Squads v4 vault [`2jBifstNwqWdrswEy8chotmVJcwfMRZkV7miqjR86qdx`](https://explorer.solana.com/address/2jBifstNwqWdrswEy8chotmVJcwfMRZkV7miqjR86qdx?cluster=devnet)
> (2-of-3). **No mainnet program exists. Not independently audited.** This is not a
> security certification and not an endorsement.
>
> The `musebook` credit-provider name is still **unavailable** in Pay / pay-kit until
> a reviewed integration lands. Related under-review PRs (not merged): pay-kit
> [#414](https://github.com/solana-foundation/pay-kit/pull/414) (devnet adapter) and
> [#413](https://github.com/solana-foundation/pay-kit/pull/413) (`*WithHooks` tags).

This program is adapted from
[`solana-foundation/payment-channels@23193ab`](https://github.com/solana-foundation/payment-channels/tree/23193ab8dc4198fd3d79a60d11371f7326f7aecb/examples/credit-provider)
`examples/credit-provider`. That revision is vendored as the git submodule
`vendor/payment-channels`, so the code that was tested is the code published here.

What changed from upstream:
- crate name `musebook_credit_provider`
- ledger PDA seed `b"musebook-credit"` (upstream uses `b"credit"`)
- callback entrypoint `b"musebook"` (upstream uses `b"credit!!"`)
- the e2e harness also prints the wire size of each transaction
- governance: config PDA, multisig-capable admin, emergency pause, per-ledger
  suspension, separate issuer via `set_issuer`, and `sol_log_data` events
  ([docs/GOVERNANCE.md](docs/GOVERNANCE.md))
- `devnet-deployment` feature trusting the devnet V2 channel `CHNLxBt…`

Docs: [GOVERNANCE](docs/GOVERNANCE.md) · [BUILD](docs/BUILD.md) · [DEVNET](docs/DEVNET.md) ·
[THREAT_MODEL](docs/THREAT_MODEL.md) · [AUDIT_READINESS](docs/AUDIT_READINESS.md)

The instruction logic and custody rules are the same as upstream, plus the
governance surface above. Hooks use the payment-channel-interface SDK
(`HookContext::decode_callback`, plus `validate_callback`, which checks the
channel account through `ChannelAccountView`).

## Submissions

This implementation is under review in the Solana Foundation's Pay repos:

- Provider-name registration (name recognized, `Unavailable` until a reviewed
  integration lands): [pay-kit #411](https://github.com/solana-foundation/pay-kit/pull/411)
- Working devnet integration, stacked on #411: [pay-kit #414](https://github.com/solana-foundation/pay-kit/pull/414)
- Merchant/provider listing: [pay-skills #337](https://github.com/solana-foundation/pay-skills/pull/337)
- Clawd Domains, a separate live x402 service from the same team: [pay-skills #336](https://github.com/solana-foundation/pay-skills/pull/336)

## Run

```sh
git clone --recursive https://github.com/themusebook/musebook-credit-provider
just test                          # cargo test --features client
SOLANA_CLI=/path/to/agave-4.3.0/bin/solana just test-credit-provider-surfpool
```

Tested with Surfpool 1.5.0, Agave CLI 4.3.0 and platform-tools v1.58 (`--arch v3`).
The Surfpool e2e run starts an offline validator on loopback and deploys with
ephemeral keys (deleted on exit). Devnet e2e evidence is in `evidence/` and
[docs/DEVNET.md](docs/DEVNET.md).

## Custody answers (from `src/processor.rs` + governance)

| Question | Answer in this code |
| --- | --- |
| Who approves spending? | The credit owner and the issuer both sign every `authorize`. It binds the payee, voucher signer, distribution hash, channel incarnation (salt + open slot), total cap, nonce and expiry slot. The owner's signature alone authorizes nothing. |
| Who is the issuer? | When `Config.issuer` is set (v2), ledger `initialize` requires that pubkey. Otherwise the legacy rule applies: issuer must equal the current upgrade authority from ProgramData. Each ledger pins its issuer at init. |
| Where does the backing live? | In the canonical SPL Token ATA owned by the ledger PDA `[b"musebook-credit", owner, mint]`. Credits go up only after the tokens actually arrive. Refunds restore credits only after the actual refund tokens are pulled back into the vault. |
| Replay prevention | Each nonce is used once and must equal the previous nonce + 1. A busy flag blocks reentry, every callback sequence is consumed, and the callback must match the stored hook commitment and all four descriptors. |
| Expiry | `expires_at_slot` blocks funding. It does not block refunds. If a channel was never funded, the owner and issuer can cancel it with `cancel_unfunded`; the nonce stays consumed. |
| Revocation | The payer can revoke the ATA delegate. A nonzero refund then fails atomically and no credits are created. A zero-refund terminal distribution still succeeds. |
| Pause | Global `set_paused` blocks `deposit` and `authorize`. Per-ledger `set_ledger_suspended` also blocks `pre_fund` / `authorize`. Refunds (`post_distribute`) and `cancel_unfunded` are never blocked. The V2 hook ABI's 4-account limit means global pause cannot attach to `pre_fund` directly (see GOVERNANCE.md). |
| Upgrade / admin | On devnet, upgrade authority and config admin are the Squads v4 vault above (2-of-3). The upgrade authority can replace the code. Rotating upgrade authority does not re-key issuers already pinned in ledgers; use `set_issuer` for new ledgers and `set_ledger_suspended` if an old issuer is compromised. |
| Recovery | No ledger close, credit redemption or migration (same limits as upstream). |

## Evidence

- Devnet program hash (solana-verify Docker build, trimmed executable): **`7055e31b…`**, matched on-chain after the Squads upgrade — `evidence/devnet-upgrade-solana-verify-2026-11-02.txt`. CI workflow `.github/workflows/verify-build.yml` rebuilds and compares to the deployed program.
- Devnet hook e2e after that upgrade: 44 PASS — same evidence file / `docs/DEVNET.md`.
- Earlier Surfpool / native-build evidence remains under `evidence/` (including `surfpool-run-2026-11-01.txt` and `local-artifact-sha256.txt`).
