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
`solana-verify build` needs Docker, and the build box had none, so the containerized hash is **not produced yet**. Run it on a machine with Docker:

```sh
cargo install solana-verify
solana-verify build --library-name musebook_credit_provider \
  --base-image solanafoundation/solana-verifiable-build:3.1.10 programs/musebook-credit-provider
solana-verify verify-from-repo -u https://api.devnet.solana.com --program-id <PROGRAM_ID> \
  https://github.com/themusebook/musebook-credit-provider --commit-hash <SHA> \
  --library-name musebook_credit_provider --mount-path programs/musebook-credit-provider -- --features devnet-deployment
```
One caveat: the program crate depends on `../../vendor/...` by path, so the mount path may have to be the repo root. Check this when you first run it.
