# Devnet deployment

> **Status (2026-11-02, ET): NOT DEPLOYED. Blocked on devnet SOL.**
> The devnet RPC faucet answered `410 "The RPC faucet is closed on this cluster. Request test SOL at https://faucet.solana.com"`. `solana airdrop` hit the rate limit on every attempt. faucet.solana.com needs an interactive GitHub sign-in and captcha, so it was not used automatically.

## Prepared deployment
| Item | Value |
|---|---|
| Cluster | devnet only (`scripts/devnet-deploy.sh` checks genesis `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG`) |
| Planned program ID | `97F68uhs5fmvddJxH1LrKytbBXrhx6ZBZwuZAdUdP7QA` (fresh throwaway keypair, kept off-repo) |
| Payer / upgrade authority / admin (stand-in, **not a multisig**) | `BcevhuQmZd4zN56tx988MRbmaGu6n1tU7UWrwkwgVata` |
| Build | `--features devnet-deployment`, sha256 `26c03af611e6cb2965181c9ad258dce2d2b0aa4dfef28438c118110c6a36e32f` |
| Needed | about 0.7 SOL for an 89 KB program, plus a few txs |
| Deploy slot / signatures | _pending_ |

To unblock: send roughly 2 devnet SOL to `BcevhuQm…Vata` (faucet.solana.com), then run
`PAYER=… PROGRAM_KEYPAIR=… scripts/devnet-deploy.sh`.

## Which payment-channels program devnet hooks need
Checked on devnet, 2026-11-02:
| ID | Size | Last deployed slot | Notes |
|---|---|---|---|
| `CHNLxYvVA28MJP9PrFuDXccuoGXAx7jBacfLEkahyGsX` (canonical, `payment_channel_interface::ID`) | 66,240 B | 480232051 | Older build, sha256 `acdb3abf…`. Not the `23193ab` V2-hooks artifact (101,312 B). |
| `CHNLxBtXw7VPqd5GMcbtVoEbn4ehyaSoiHj56Ao5sWkJ` (`devnet-deployment`) | 101,312 B | 511497054 | Same size as our local `23193ab` V2 build. Not byte-identical to our rebuild: dump `3e94032d…` vs local `538e8b3f…`. Authority `4zTeC5mV…`. |

So the provider gained a `devnet-deployment` feature that trusts `CHNLxBt…` callbacks. Whether `CHNLxBt…` exposes the exact `23193ab` hook ABI is **unverified** (question for maintainers).

## End-to-end status
- **Local Surfpool 1.5.0 (real SBF, real signatures): 55 PASS**, including the new governance cases. See `evidence/surfpool-run-2026-11-02-governance.txt`. The governance cases: init config gated by the upgrade authority, a non-admin pause rejected, pause blocking deposit and authorize, a suspended ledger rejecting `pre_fund` inside a real channel `open`, and unpause or lift restoring the flow.
- **LiteSVM SBF governance tests: 4/4 pass** (`tests-svm`).
- **Devnet e2e: not run** (no SOL). Also remaining: the e2e harness is loopback-only and seeds accounts with Surfpool airdrops. A devnet mode has to fund from the payer and point the channel at `CHNLxBt…`.
- **pay-kit / pay:** they don't emit `*WithHooks` channel instructions yet, so a merchant-driven hook flow can't be exercised through pay-kit on any cluster. The e2e builds the channel instructions directly.
