# Governance, admin authority and pause

## Accounts
| Account | Seeds | Size | Fields |
|---|---|---|---|
| `Config` | `[b"musebook-config"]` | 99 B (v2); legacy v1 is 67 B | `version u8`, `bump u8`, `paused bool`, `admin [u8;32]`, `pending_admin [u8;32]`, `issuer [u8;32]` (v2) |
| `Ledger` | `[b"musebook-credit", owner, mint]` | 332 B | as upstream, plus `suspended bool` after `active` |

## Instructions (tag byte)
| Tag | Name | Signers | Accounts | Effect |
|---|---|---|---|---|
| 4 | `initialize_config(admin: [u8;32])` | payer, **current upgrade authority** | payer, config, program, programdata, upgrade authority, system | Creates `Config` once. Front-run-proof: the upgrade authority is read from loader-owned ProgramData. |
| 5 | `set_paused(u8)` | admin | admin, config | Global pause flag. |
| 6 | `propose_admin([u8;32])` | admin | admin, config | Sets `pending_admin`; all-zero cancels. |
| 7 | `accept_admin` | pending admin | pending admin, config | Completes two-step transfer. |
| 8 | `set_ledger_suspended(u8)` | admin | admin, config, ledger | Per-ledger stop (see below). |
| 9 | `set_issuer([u8;32])` | admin | admin, config (w), payer (s,w), system | Sets the issuer that co-signs ledger `initialize`. All-zero restores the legacy upgrade-authority rule. Migrates a v1 config to v2 in place, with the payer funding the extra rent. It affects only new ledgers, since each ledger stores its issuer at init. |

Custom errors: `0x4d420001` PAUSED, `…02` UNAUTHORIZED, `…03` BAD_CONFIG, `…04` NO_PENDING_ADMIN, `…05` SUSPENDED, `…06` INVALID_ADMIN.

## What pause blocks
- **Global pause** blocks `deposit` and `authorize` (no new credit or new hook approval). `deposit` and `authorize` now take the config PDA as their last account.
- **The global flag cannot reach the `pre_fund` callback.** The V2 hook ABI allows at most 4 extra accounts per descriptor and the provider already uses all 4 (ledger, vault ATA, payer ATA, token program). `validate_callback` enforces an exact account count. We did not change the hook interface. So an authorization that was already active before the pause could still fund, bounded by its `cap` and `expires_at_slot`.
- **`set_ledger_suspended`** closes that gap per ledger. `pre_fund` (open/top-up) rejects with SUSPENDED, and so does `authorize`. Incident procedure: pause globally, then suspend every ledger with `active == true` (use `getProgramAccounts` with a `dataSize = 332` filter).
- **Never blocked:** `post_distribute` (refunds back into custody and credit restoration) and `cancel_unfunded`. A pause must not trap user funds.
- Open question for upstream: a descriptor flag or a 5th account slot would let the global pause gate `pre_fund` directly.

## Using a Squads v4 multisig as admin
Program `SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf` (same ID on devnet and mainnet).
1. Create the multisig (Squads app or `@sqds/multisig` `multisigCreateV2`). Choose threshold and members (e.g. 2-of-3 hardware keys).
2. Derive the **vault PDA**: `getVaultPda({ multisigPda, index: 0 })`. Use the vault, **not** the multisig account. The vault is what signs in CPI.
3. Current admin: `propose_admin(vault_pda)`.
4. In Squads, create a vault transaction containing `accept_admin` with accounts `[vault_pda (signer), config (writable)]`. Approve it to threshold, then `vaultTransactionExecute`. Squads signs as the vault PDA.
5. Check: `Config.admin == vault_pda`, `pending_admin == 0`.
6. Upgrade authority: `solana program set-upgrade-authority <PROGRAM> --new-upgrade-authority <vault_pda>`. Future upgrades go through a Squads program-upgrade transaction (buffer + `BPFLoaderUpgradeable::Upgrade` executed by the vault).

The program has no Squads-specific code. Any signer works, and that is intentional.

**Devnet (done 2026-11-02):** program `97F68uhs…` is now governed by Squads v4 multisig `9nebqfSd37E3YyodB6iEMz18JjWP3QoydporvedkjDw2`, a 2-of-3. Its vault `2jBifstNwqWdrswEy8chotmVJcwfMRZkV7miqjR86qdx` holds both admin and upgrade authority. Steps 3 to 6 above were run by `scripts/squads-devnet/run.js`. Signatures are in docs/DEVNET.md. Pause and unpause were executed as vault transactions, and a direct pause by the old deployer key failed with UNAUTHORIZED. Two of the three devnet member keys live on one box, so this proves the mechanics, not key separation. Mainnet members must be hardware wallets you choose.

## Issuer vs upgrade authority
Ledger `initialize` (config PDA as its 8th account) checks the issuer against `Config.issuer` when it is set. If it isn't set, it falls back to the legacy rule (issuer == current upgrade authority, read from ProgramData). Rotate the issuer with `set_issuer`, run through Squads. Ledgers that already exist keep the issuer they were created with: `authorize` and `cancel_unfunded` check `Ledger.issuer`. Issuer rotation therefore doesn't revoke old ledgers. Use `set_ledger_suspended` on them if the old issuer key is compromised.
On devnet (2026-11-02), the issuer is the throwaway `8CBpS8SE5SpasbvS6LL8Y3JqWYDdAmVL9MhwPHCGuX1v`, set through Squads vault tx #5.
