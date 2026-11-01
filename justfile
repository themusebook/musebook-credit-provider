# Unit/interface tests (host).
test:
    cargo test --manifest-path programs/musebook-credit-provider/Cargo.toml --features client

# Offline Surfpool e2e: Surfpool 1.5.0, Agave CLI 4.3.0, platform-tools v1.58 (--arch v3).
test-credit-provider-surfpool:
    CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}/credit-e2e-channel-sbf" cargo build-sbf --tools-version v1.58 --arch v3 --manifest-path vendor/payment-channels/program/payment_channels/Cargo.toml --sbf-out-dir target/deploy-v2
    CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}/credit-e2e-provider-sbf" cargo build-sbf --tools-version v1.58 --arch v3 --manifest-path programs/musebook-credit-provider/Cargo.toml --sbf-out-dir target/deploy
    bash e2e/run.sh
