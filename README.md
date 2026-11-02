# musebook-credit-provider

API credit provider for [musebook.trade](https://musebook.trade), built for Pay's
credit-provider model (preFund / postDistribute hooks on payment-channel V2).

> **Status: pre-audit test code. Not deployed** (devnet deploy prepared, blocked on
> faucet SOL; see docs/DEVNET.md). No mainnet program exists. This is not a security certification and not an endorsement.
> Pay and pay-kit don't emit `*WithHooks` channel instructions yet, so the
> `musebook` provider name stays **unavailable** in both of them.

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
  suspension and `sol_log_data` events (docs/GOVERNANCE.md)
- `devnet-deployment` feature trusting the devnet V2 channel `CHNLxBt…`

Docs: [GOVERNANCE](docs/GOVERNANCE.md) · [BUILD](docs/BUILD.md) · [DEVNET](docs/DEVNET.md) ·
[THREAT_MODEL](docs/THREAT_MODEL.md) · [AUDIT_READINESS](docs/AUDIT_READINESS.md)

The instruction logic and custody rules are the same as upstream. Hooks use the
payment-channel-interface SDK (`HookContext::decode_callback`, plus `validate_callback`, which checks the channel account through
`ChannelAccountView`).

## Run

```sh
git clone --recursive https://github.com/Solizardking/musebook-credit-provider
just test                          # cargo test --features client
SOLANA_CLI=/path/to/agave-4.3.0/bin/solana just test-credit-provider-surfpool
```

Tested with Surfpool 1.5.0, Agave CLI 4.3.0 and platform-tools v1.58 (`--arch v3`).
The e2e run starts an offline Surfpool on loopback and deploys with ephemeral
keys, which are deleted on exit.

## Custody answers (from `src/processor.rs`)

| Question | Answer in this code |
| --- | --- |
| Who approves spending? | The credit owner and the issuer both sign every `authorize`. It binds the payee, voucher signer, distribution hash, channel incarnation (salt + open slot), total cap, nonce and expiry slot. The owner's signature alone authorizes nothing. |
| Who is the issuer? | At `initialize`, the issuer must be the program's upgrade authority, read from ProgramData. It is pinned in each ledger after that. |
| Where does the backing live? | In the canonical SPL Token ATA owned by the ledger PDA `[b"musebook-credit", owner, mint]`. Credits go up only after the tokens actually arrive. Refunds restore credits only after the actual refund tokens are pulled back into the vault. |
| Replay prevention | Each nonce is used once and must equal the previous nonce + 1. A busy flag blocks reentry, every callback sequence is consumed, and the callback must match the stored hook commitment and all four descriptors. |
| Expiry | `expires_at_slot` blocks funding. It does not block refunds. If a channel was never funded, the owner and issuer can cancel it with `cancel_unfunded`; the nonce stays consumed. |
| Revocation | The payer can revoke the ATA delegate. A nonzero refund then fails atomically and no credits are created. A zero-refund terminal distribution still succeeds. |
| Pause | **None.** There is no pause instruction. |
| Upgrade | The upgrade authority can replace the code, so it is fully trusted. Rotating it does not change the issuer pinned in existing ledgers. The governance model is **TODO**. |
| Recovery | No ledger close, credit redemption or migration (same limits as upstream). |

## Evidence (local, 2026-11-01)

- `evidence/surfpool-run-2026-11-01.txt`: 43 PASS lines, with signatures, wire bytes and CU (Surfpool-local signatures, not on any public cluster).
- `evidence/local-artifact-sha256.txt`: SHA-256 of this box's local builds. Reproducible-build verification (e.g. solana-verify in a pinned container) is **TODO**.
