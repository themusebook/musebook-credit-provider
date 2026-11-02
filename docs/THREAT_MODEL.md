# Threat model: musebook credit provider

Scope: this program (custody of the credit-backing SPL tokens in a ledger-owned vault ATA, credit accounting, V2 hook callbacks) and its governance. The payment-channels program is out of scope here and trusted as the callback source. Status: pre-audit, devnet-only.

## Assets
Vault ATA balances (real tokens backing credits) · owner payer-ATA delegations (capped) · ledger accounting (credits, funded, refunded, nonce) · Config (admin, pause) · upgrade authority.

## Actors and keys
| Key | Powers | If compromised | Mitigation |
|---|---|---|---|
| **Issuer** | Co-signs ledger `initialize`, `authorize` and `cancel_unfunded`. It is `Config.issuer`, set by the admin via `set_issuer`. If unset, it falls back to the upgrade authority. | Can authorize channels to any payee **only with the owner's co-signature**, bounded by cap, nonce and expiry. Cannot move vault funds alone. | Owner + issuer co-sign; `cap` must equal the exact delegate amount; nonces are single-use; expiry. Separate `Config.issuer` (done in v2). Hold it in an HSM. On compromise: `set_issuer` to a new key and suspend the ledgers that used the old one. |
| **Upgrade authority** | Replaces the program and can drain every vault. | **Total loss.** | Squads v4 multisig vault (docs/GOVERNANCE.md), hardware signers, published verifiable build hash, and a timelocked upgrade policy. On devnet it is a single throwaway key (stand-in). |
| **Admin** (`Config.admin`) | Pause/unpause, suspend ledgers, hand over admin. | Can **halt new activity** (DoS). Cannot move funds: pause never blocks refunds or `cancel_unfunded`. | Squads vault; two-step handover; events on every change. |
| **Owner** | Deposits, co-signs authorize, signs channel txs. | Loses only their own credits. | Per-ledger PDA; delegation capped. |

## Threats
1. **Malicious or forged callback.** Mitigated: `validate_callback` checks the CPI comes from `CHANNEL_PROGRAM`, the exact account count and aliasing, descriptor commitment, channel PDA, payee/signer/distribution hash, monotonic `sequence`, and refund destination. Provenance is pinned per build flavor (canonical vs `devnet-deployment`). A wrong flavor deployed means hooks never validate: it fails closed.
2. **Reentrancy.** `busy` flag set across token CPIs; `load` rejects a busy ledger. Balance post-conditions are checked after every transfer.
3. **Replay / stale authorization.** Nonce must be `prev+1`; `cancel_unfunded` keeps the nonce; channel incarnation is bound via `open_slot` and `salt`; expiry slot.
4. **Compute limits.** Measured worst case 118,845 CU on Surfpool and **165,335 CU on devnet** (open with pre_fund, including ATA creation) inside a 200k default budget. The devnet margin is only about 17%. The hook runs inside the channel instruction, so the channel's own CU plus the hook must fit. Merchants should set an explicit `ComputeBudget` limit. Repeated PDA bump searches make CU depend on the key set, and one Surfpool `open` exceeded 200k. The harness now requests 400k. Event logging adds a few hundred CU. Watch the payment-channels `docs/security/compute-budget-caps.md`. A CU-exhaustion failure fails the whole channel tx atomically (no partial state).
5. **Fee-payer drain.** The program never pays fees or rent for others, except that `initialize` and `initialize_config` charge rent to their signer. Fee-payer risk sits with the pay-kit server or facilitator that co-signs or pays for client-built txs (see payment-channels `docs/security/fee-payer-drain.md`). Facilitators must simulate, allow-list program IDs (this program + channel + token + ATA + compute budget), cap CU price, and never sign txs that include unexpected writable accounts they pay rent for.
6. **Pause semantics.** Global pause blocks `deposit` and `authorize`. It **cannot** gate `pre_fund` (all 4 hook account slots are used and the hook ABI was kept intact), so active authorizations need `set_ledger_suspended`. That is a residual risk window during an incident: the operator must enumerate active ledgers. Refund paths stay open on purpose.
7. **Front-running governance init.** `initialize_config` requires the current upgrade authority's signature (read from ProgramData).
8. **Admin handover mistakes.** Two-step propose/accept, with zero-cancel.
9. **Token account games.** Vault and payer must be canonical ATAs, not frozen or native, with no close authority. The vault has no delegate. Frozen-mint or Token-2022 extensions are not supported (Token program only).
10. **No ledger close / migration.** Rent stays locked and there is no redemption path for unspent credits except as channel funding. Accepted for devnet; must be resolved before mainnet (see AUDIT_READINESS).

## Incident runbook (short)
1. Admin (Squads): `set_paused(1)`. 2. Enumerate `active` ledgers and call `set_ledger_suspended(1)` on each. 3. Let in-flight channels settle; refunds continue. 4. Fix, then publish a verifiable build and upgrade via Squads. 5. Lift suspensions, then `set_paused(0)`. Every step emits a `musebook:v1` event.
