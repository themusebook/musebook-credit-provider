# Audit readiness

## Ready
- Scope is frozen per commit. Small program (~1.1k LoC Rust, Pinocchio, no_std). The diff against upstream `payment-channels@23193ab examples/credit-provider` is readable (seed, entrypoint, governance, events, ledger `suspended`).
- Separate issuer via `set_issuer`, with v1→v2 config migration, deployed to devnet through a Squads upgrade proposal (hash verified).
- Squads v4 2-of-3 vault is admin and upgrade authority on devnet, with pause and unpause proven through vault transactions.
- Governance: config PDA, admin (multisig-capable), two-step handover, global pause, per-ledger suspension, `sol_log_data` events.
- Devnet deployment `97F68uhs…` (slot 517910524) whose on-chain ELF matches the verifiable build hash, plus a full hook e2e on devnet (62 PASS, docs/DEVNET.md).
- Tests: host unit tests (7 + 3 interface), LiteSVM SBF governance tests (4), Surfpool real-SBF e2e (61 PASS) covering fund/topUp/seal/distribute/refund/withdraw/cancel/expiry/replay/cap/rollback plus pause/suspend.
- Reproducible build script with pinned platform-tools, lockfile and submodule rev. Deterministic hash verified locally.
- Threat model (docs/THREAT_MODEL.md) and governance doc.

## Not ready
- **solana-verify Docker hash is enforced in CI** against the deployed devnet program (`7055e31b…`); a code change fails CI until a matching Squads upgrade ships. Mainnet still has no program and no verify target.
- **Global pause cannot gate `pre_fund`** without a hook-ABI change. Relies on per-ledger suspension. Needs a maintainer decision.
- **Issuer rotation does not re-key existing ledgers.** Each ledger keeps its init-time issuer. Revoking it relies on suspension.
- **CU variance:** PDA bump searches repeat on every callback, so a hooked `open` can exceed the 200k default (seen once on Surfpool). Clients must set a CU limit. Fix: store bumps in the ledger.
- **No ledger close / credit redemption / migration path.**
- **Devnet `CHNLxBt…` not byte-verified** against `23193ab`.
- **No fuzzing or property tests.** No formal CU budget test in CI (CU only observed in e2e logs).
- **Surfpool e2e not in CI** (CI runs host + SBF build + LiteSVM).
- **No SECURITY.md contact, published upgrade policy or timelock.**
- **No independent audit.** The upstream example's audit status does not carry over.
- **pay-kit adapter is under review, not merged**: #414 (devnet-only adapter, ready for review), #413 (`*WithHooks` tags, approved), #411 (name reservation, still draft). No conformance vectors or merchant demo yet. Mainnet unavailable.
