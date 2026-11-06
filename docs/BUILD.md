# Reproducible build

```sh
git submodule update --init          # vendor/payment-channels @ 23193ab
scripts/build-verifiable.sh default  # canonical channel ID build
scripts/build-verifiable.sh devnet   # --features devnet-deployment (trusts CHNLxBt…)
```

Pinned: platform-tools **v1.58** (`--tools-version v1.58 --arch v3`), `Cargo.lock` with `--locked`, and the vendored payment-channels at `23193ab`. The script refuses any other submodule revision. Tested with cargo-build-sbf from Agave 3.1.10. Host tests use stable Rust (`rust-toolchain.toml`). The SBF compiler is the one inside platform-tools, so the host toolchain does not affect the `.so`.

Hashes below are for v2 (set_issuer). v1 was default `f8570c12…`, devnet `26c03af6…`. Determinism check (2026-11-02): two builds from empty target dirs produced the same bytes.

| Flavor | sha256 |
|---|---|
| default | `157a0ebded8ea5c73aaadc956e96d1110e4214b142a09f1b31a2566bda599838` |
| devnet (`devnet-deployment`) | `4af58a20ee217e920cd050f311f8f99b86ffedd075a07f482f7aad8f41f7c62e` |

## solana-verify
CI (`.github/workflows/verify-build.yml`) runs `solana-verify` in Docker on every
push/PR and **fails if the Docker executable hash differs from the deployed
devnet program** `97F68uhs…`. The Squads-upgraded on-chain executable hash is
`7055e31b22e4055f6ea0e488e9b086de523f0d3dbaa776f59d2df3ca85f14e7b` (see
`evidence/devnet-upgrade-solana-verify-2026-11-02.txt`).

Native `scripts/build-verifiable.sh` hashes in the table above are a separate,
non-Docker toolchain path and are **not** expected to equal the Docker /
on-chain hash.

Local Docker rebuild (needs Docker):

```sh
cargo install solana-verify --version 0.5.2 --locked
# Prefer the workflow's pinned base image + args; see verify-build.yml.
```
