# Governance, admin authority and pause

## Accounts
| Account | Seeds | Size | Fields |
|---|---|---|---|
| `Config` | `[b"musebook-config"]` | 67 B | `version u8`, `bump u8`, `paused bool`, `admin [u8;32]`, `pending_admin [u8;32]` |
| `Ledger` | `[b"musebook-credit", owner, mint]` | 332 B | as upstream, plus `suspended bool` after `active` |

## Instructions (tag byte)
| Tag | Name | Signers | Accounts | Effect |
|---|---|---|---|---|
| 4 | `initialize_config(admin: [u8;32])` | payer, **current upgrade authority** | payer, config, program, programdata, upgrade authority, system | Creates `Config` once. Front-run-proof: the upgrade authority is read from loader-owned ProgramData. |
| 5 | `set_paused(u8)` | admin | admin, config | Global pause flag. |
| 6 | `propose_admin([u8;32])` | admin | admin, config | Sets `pending_admin`; all-zero cancels. |
| 7 | `accept_admin` | pending admin | pending admin, config | Completes two-step transfer. |
| 8 | `set_ledger_suspended(u8)` | admin | admin, config, ledger | Per-ledger stop (see below). |

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

**Devnet stand-in:** the devnet deployment uses one throwaway key as upgrade authority and admin. It is not a multisig. Do not read devnet governance as production custody.

## Caveat: issuer == upgrade authority at ledger init
`initialize` (ledger) still requires the issuer to equal the *current upgrade authority* (upstream design). Once the upgrade authority moves to a multisig, every new ledger needs a multisig signature. Before mainnet, decide between two options: keep that (slow but strict), or add a `Config.issuer` that the admin sets, and check against it instead. Tracked in AUDIT_READINESS.md.
