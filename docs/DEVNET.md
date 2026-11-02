# Devnet deployment

> **Status (2026-11-02, ET): DEPLOYED on devnet. Full hook e2e passed (62 PASS, 44 confirmed signatures).**
> Devnet only. Pre-audit. The upgrade authority and admin are now a **Squads v4 2-of-3 vault** (see below).

## Deployment
| Item | Value |
|---|---|
| Program ID | [`97F68uhs5fmvddJxH1LrKytbBXrhx6ZBZwuZAdUdP7QA`](https://explorer.solana.com/address/97F68uhs5fmvddJxH1LrKytbBXrhx6ZBZwuZAdUdP7QA?cluster=devnet) |
| ProgramData | `4cR7uYdQEw3QcxzbBpVFannRLZdpgqpvhkiPUV7zNo9k` (max-len 96,000 B, rent 0.48855884 SOL) |
| Deploy slot | **517910524** |
| Deploy signature | [`3NcFQoDxDcoMtDWWoLP37kW1nTMoQwnska9KeuyNLUp8ExgKwEkfe1ZnpAQo72qtNstyepYkALZM6GXs2JNSs7XX`](https://explorer.solana.com/tx/3NcFQoDxDcoMtDWWoLP37kW1nTMoQwnska9KeuyNLUp8ExgKwEkfe1ZnpAQo72qtNstyepYkALZM6GXs2JNSs7XX?cluster=devnet) |
| Upgrade authority = issuer = admin | **Now the Squads v4 vault `2jBifstNwqWdrswEy8chotmVJcwfMRZkV7miqjR86qdx` (2-of-3)**. Originally the deployer `BcevhuQmZd4zN56tx988MRbmaGu6n1tU7UWrwkwgVata`; see the Squads section. |
| Config PDA init | [`3hFADYYwjshECgytsxTv77gNzq6RySSqaYNRs87ArEonVt3k3YnZvFwqo3ZnrqvmLzbgJoaRaRd2ufu8pQkvZRAi`](https://explorer.solana.com/tx/3hFADYYwjshECgytsxTv77gNzq6RySSqaYNRs87ArEonVt3k3YnZvFwqo3ZnrqvmLzbgJoaRaRd2ufu8pQkvZRAi?cluster=devnet) |
| Channel program (hooks) | `CHNLxBtXw7VPqd5GMcbtVoEbn4ehyaSoiHj56Ao5sWkJ` (`devnet-deployment`) |
| Build | `scripts/build-verifiable.sh devnet` sha256 `26c03af611e6cb2965181c9ad258dce2d2b0aa4dfef28438c118110c6a36e32f` |
| On-chain check | `solana program dump` output, first 89,280 bytes: sha256 **`26c03af6…6e32f` (match)**. The remaining 6,720 bytes are zero padding from `--max-len`. |
| SOL funding | 1.28 devnet SOL from the user (tx `5eK7pEiB…69SJMR`). No faucet was used. |

Deploy notes: a default deploy (max-len 2× = 178,560 B) needs about 0.91 SOL of ProgramData rent plus a 0.45 SOL buffer at the same time (about 1.36 SOL), which is more than 1.28 SOL. With `--max-len 96000` it needed about 0.49 + 0.45 SOL (the buffer is refunded after deploy). Agave CLI 3.1.10 refuses SBPF v3 ELFs locally ("sbpf_version … not enabled"), so `devnet-deploy.sh` uses `SOLANA_CLI` (4.3.2). The first attempt failed before sending any transaction, and no buffer was left behind (`program show --buffers` is empty). Upgrades beyond 96,000 B need `solana program extend`.

## E2E against devnet
`CREDIT_E2E_PAYER=… CREDIT_E2E_PROGRAM=97F68… e2e/run-devnet.sh`. It is the same harness as Surfpool, built with `--features devnet`: channel `CHNLxBt…`, devnet treasury owner, and accounts funded by system transfer from the deployer instead of airdrops. It checks the devnet genesis hash first.

Covered for real on devnet: initialize negatives (impostor issuer, missing signature, substituted ProgramData) · governance config init · pause/unpause (non-admin rejected, deposit and authorize blocked while paused) · ledger suspension (`pre_fund` rejects a real channel `open`) · two-step admin handover and back (non-pending accept rejected, old admin rejected) · deposit · authorize · **open with `pre_fund` hook** · cap-overrun rollback · **topUp with hook** · voucher + seal · **distribute with `post_distribute` refund** · duplicate distribute · consumed-nonce replay · withdrawPayer · zero-refund terminal · expiry reject · cancel_unfunded · canceled-nonce replay · fresh nonce.
Max observed: 165,335 CU (open with `pre_fund`), versus 118,845 on Surfpool.

Full log: `evidence/devnet-run-2026-11-02.txt` (each line has the full signature). Rejected cases submit with `skipPreflight`, fail on-chain, and are verified by error code plus an unchanged account snapshot.

| Step | Signature | CU |
|---|---|---|
| initialize governance config (upgrade-authority gated) | [`3hFADYYwjshECgyt…`](https://explorer.solana.com/tx/3hFADYYwjshECgytsxTv77gNzq6RySSqaYNRs87ArEonVt3k3YnZvFwqo3ZnrqvmLzbgJoaRaRd2ufu8pQkvZRAi?cluster=devnet) | 18207 |
| fund from devnet deployer | [`J7cKhXWHVAgwr7VM…`](https://explorer.solana.com/tx/J7cKhXWHVAgwr7VMcH8VYJokDLKKWYU3qTA4w3qSfroYTxXMgCrLEUVT1DcAmagJumyqx74mpjfNpDLFrNgCQre?cluster=devnet) | 150 |
| create SPL mint | [`5fEYFdppej41Qsh9…`](https://explorer.solana.com/tx/5fEYFdppej41Qsh9WGPwadn7BCmb7dAHDfXBK7f3ihpBurkHSnH9LbK6dRa2ja3VehmYEUNLTAa34UBztPgCwkGM?cluster=devnet) | 352 |
| create associated token account | [`3sF8domKwVoTxvav…`](https://explorer.solana.com/tx/3sF8domKwVoTxvavo5x5CkpsDCNXvQA7ArPCRbHUdopApJjFzhBNRK48GBpFPGsmbxF2qw9q4ZPSyBY12FYmbqSo?cluster=devnet) | 22417 |
| create associated token account | [`5Frvy18c9Ed2G2io…`](https://explorer.solana.com/tx/5Frvy18c9Ed2G2ioui4hTQG9v2eHgjdPEzUMFfBQdjJGpvCGoG1Jztjnpqz4woGjAfNp1hygvXZqSY1HyjBECXaH?cluster=devnet) | 15018 |
| create associated token account | [`7u3hnaqnHUy5j3eZ…`](https://explorer.solana.com/tx/7u3hnaqnHUy5j3eZV69DCtEcL3P6AecbEtDvyxArkrpsT6JSujcTac6ZeSNA3DUribG8ynyeZGo8WMvwevLUbih?cluster=devnet) | 15018 |
| create associated token account | [`5uwP6eCGAw53D8Py…`](https://explorer.solana.com/tx/5uwP6eCGAw53D8PyzfK5fRYuUgFswFk88Sqry9ffc8J383Nqqh1GZroMAfnqcaB7Py7vRpHP8uFt5kyNoz332JNS?cluster=devnet) | 15018 |
| mint backing tokens | [`25vqbt95WqLDEZG1…`](https://explorer.solana.com/tx/25vqbt95WqLDEZG1qhfpREGcxbUrVyDBCPySSFJFynL2u7TWWJnS3X2dnBwzdVfyv9sTP6aF4HZvKHuHkp7D6aXG?cluster=devnet) | 120 |
| prefund ledger PDA with zero-data rent minimum | [`3d9Zw8okJMoDYFHe…`](https://explorer.solana.com/tx/3d9Zw8okJMoDYFHeushnqs52EVCC1DJvQaZ5VjiqtqDPpcxaKf8SSnDzfbnZPM6djr7eSNQTdBQHmEmUj59SRkCs?cluster=devnet) | 150 |
| initialize prefunded ledger with real upgrade authority | [`5n8Th1iMfFREe5hZ…`](https://explorer.solana.com/tx/5n8Th1iMfFREe5hZ13Rm4hAF3yVSCAPBpAqeW6Dyad9xLySRTVpxuR3QSZy8Mkrez3UQ81Sq6hD7YsrXnYrvwkQZ?cluster=devnet) | 7831 |
| admin pauses | [`3rnAhtBvVzA1STBR…`](https://explorer.solana.com/tx/3rnAhtBvVzA1STBRXHjrSRCPgSQNaJ4Y1q1XTWHrjeZ1Hx3tYGLAJ3Ns9ZFVg9bSvT1a3wwHr4aASRapnE4QDrYt?cluster=devnet) | 14761 |
| admin unpauses | [`2LbD23Vj1UPHLwNX…`](https://explorer.solana.com/tx/2LbD23Vj1UPHLwNXgS5nL9HLbAfz6GcHXu6UUcvptMrYq1HnrkCqPEtSUUWApdKXqjfpqU9AanHjm7CaQ9AuNit7?cluster=devnet) | 14761 |
| deposit real SPL backing | [`42YgdZuwxBLBRrqQ…`](https://explorer.solana.com/tx/42YgdZuwxBLBRrqQfsNb5HqVt78QFQBEzm7zL7SXDdUPUf7hQkvqtZ6AnjP5Rc74zpzrisSefuV7WH5s1NsMndvp?cluster=devnet) | 35536 |
| admin pauses again | [`acBzd1emZgasAC4c…`](https://explorer.solana.com/tx/acBzd1emZgasAC4cLr7Zj2Zhs36poPMjCzMomPPaEbGnYv1zBTqR3cHdTMC85Rg86sgCyxVUopGjWtKuVU7TTgt?cluster=devnet) | 14761 |
| approve bounded refund delegate | [`2zS4z4jbHb2nUvAR…`](https://explorer.solana.com/tx/2zS4z4jbHb2nUvARpSsE6GSh9amJCJLpTejiWSYXMy1o1YWZr3LSHoFw1rrHEQTcsVn7Sx7QAwA8ruGTnwTSMRfc?cluster=devnet) | 123 |
| admin unpauses | [`4LXqsnUncU7rnCLH…`](https://explorer.solana.com/tx/4LXqsnUncU7rnCLHGMKd8q5ZGcLiBUtzmzRMBjTTihxBu2S43bYRAgi6diiSg6i6rP3EkYZosSU1a3Tt2jpuNNYh?cluster=devnet) | 14761 |
| approve bounded refund delegate | [`21B4fT8y1C3bD1Cj…`](https://explorer.solana.com/tx/21B4fT8y1C3bD1Cjvid174SHy8NEGYSoCWR9Td3tDqpsJWZQNDABCKmMTH98LeYyZH1e9AQLXeVVqWs6UMxe8exM?cluster=devnet) | 123 |
| owner + issuer authorize one incarnation | [`2cKumi8iN2jCEs8e…`](https://explorer.solana.com/tx/2cKumi8iN2jCEs8eNDLXZNF28Hhk4uB3Bth8XGbrNZ5bnw7gw1SokDaRKyEmmFCbvvx9V759LdTAfU5oh7d3gjq1?cluster=devnet) | 56240 |
| admin suspends ledger | [`3PEJ3cDWxWjTN5Ep…`](https://explorer.solana.com/tx/3PEJ3cDWxWjTN5EpazVAFXBLJWuLiU4ZDAZrF5yVJAxsbPzDR4Ww6iEnK2G7xaNBzzuV3xMmxF4wZuo9n5SuVehC?cluster=devnet) | 19469 |
| admin lifts ledger suspension | [`3iHZzrfzuuqcLhcb…`](https://explorer.solana.com/tx/3iHZzrfzuuqcLhcb48twQVgvN5rUCAmD2WAgQXioDGDrMZyCuHdH9nSNUW6daKDNhfWWRScoBvLsha8tiVkfFFzz?cluster=devnet) | 19469 |
| admin proposes new admin | [`5tXCou2HFAW2SWMY…`](https://explorer.solana.com/tx/5tXCou2HFAW2SWMYVgXJAbnNMfzYggGAHGMgAYD1hqfBd68MzhvaFJvkUaskzGV7vP7XRxJqHampaBe4En9GPiEu?cluster=devnet) | 14803 |
| pending admin accepts | [`61VBJZnErRt39UVt…`](https://explorer.solana.com/tx/61VBJZnErRt39UVteWKrfHXXpUHdRjdq1eeBYF1NBUgFjrmJMMVDqtjfZsVwzAXWD8Fdej6pc5JeooriaLxc3Pfv?cluster=devnet) | 14806 |
| new admin proposes handback | [`3NuB1VWc7W7yy858…`](https://explorer.solana.com/tx/3NuB1VWc7W7yy858iT6bjNmJvGdfSudT8th2tV5pJaUEZNz8b5PzTP2PercUnZehReeAfea7n6yfAoXEKLhnHNXr?cluster=devnet) | 14803 |
| original admin accepts handback | [`3RW6V2aYUeP9BTau…`](https://explorer.solana.com/tx/3RW6V2aYUeP9BTauCjU52cCLHV7SqadX9ed8q5ZWKGueatfJ1CdHT3FBzGD4VmNTg3U8pyTrAwCsnbsaydGy63YG?cluster=devnet) | 14806 |
| open consumes 1000 backed credits | [`64rPxNoPxsSxz3Yc…`](https://explorer.solana.com/tx/64rPxNoPxsSxz3YcyWYeJ8d71Qr4gBPAYxAVua5AzffHPPudyxFRYR51Yy9hPis3Eo4bDosnc3baTDiE8WWz2CJJ?cluster=devnet) | 165335 |
| topup consumes another 200 credits | [`62XfXdsLMkJSv71q…`](https://explorer.solana.com/tx/62XfXdsLMkJSv71qcZZ4FXq4BzXoP61wQod6HKSoHuK29F6SrFmWjpuQceSGmrnNpUwQ5vbm37dfhA9m8szW73nn?cluster=devnet) | 134720 |
| signed voucher + cooperative seal | [`XHL9zaCF3wE1QE1e…`](https://explorer.solana.com/tx/XHL9zaCF3wE1QE1eQzLVmMBgDc5RfBwge1S968ZPHKVDMpLWE9YRXaghGgjb1wysoE66tWX6QgBbHCx3apA3Kv2?cluster=devnet) | 513 |
| revoke refund delegate | [`3iosCvH5oRMLfhYZ…`](https://explorer.solana.com/tx/3iosCvH5oRMLfhYZrho19w6FLrMnJpRoZB8J1FjcQoGpF77nk8HnwJveHVpNr1jDKMtvomvMe4j7ZyhFfnDgrzU9?cluster=devnet) | 102 |
| approve bounded refund delegate | [`2fwYwYLjYqeiaLcd…`](https://explorer.solana.com/tx/2fwYwYLjYqeiaLcdCSyVjDXe9EqfryyZ5En23U74zxCNV6zykfTco2j58888boxjKpCp6qEojNXDDjCccFNDm9r8?cluster=devnet) | 123 |
| refund 500 tokens back into custody and credits | [`5h6yzjufWK931bKA…`](https://explorer.solana.com/tx/5h6yzjufWK931bKAx4tDHdagRBnSiHzhuQmSMJDb7xF9smAEymr5REX9c7nR8pFpv9GyujaHVqXSeukYPwPgdAsg?cluster=devnet) | 161756 |
| approve bounded refund delegate | [`4Mv8vrUwmSqYpFrL…`](https://explorer.solana.com/tx/4Mv8vrUwmSqYpFrLQSEhvcrRFV4h2GFa2YxmdojeC9due1DoYHbyAaTEw4ukx3gwZ1w28aPX8meYGJZSeCbvWn78?cluster=devnet) | 123 |
| approve bounded refund delegate | [`GguTQUiq76j781B5…`](https://explorer.solana.com/tx/GguTQUiq76j781B5zjdRY4UBVjYXbPtJUvpCyk7jnJ3cp6GU5fi7YoQDpRumpXsiugu8dMaQwtrx83Q5nYTNzoD?cluster=devnet) | 123 |
| owner + issuer authorize one incarnation | [`4nZX9upVBkZDWD7W…`](https://explorer.solana.com/tx/4nZX9upVBkZDWD7Wt7q1RnNVqz5hLaqZbGX6pGud9bjDxTTWXapRz2FYAew22dK77Q2ay8HmGarfPA8r9deYYhsm?cluster=devnet) | 54740 |
| open second credit-backed channel | [`3pMuQcs4X299FbLX…`](https://explorer.solana.com/tx/3pMuQcs4X299FbLX6T2XXUXtXEambARZNsRjRZ1Q1dB2YGNHukFkhZaM56MQ3TxvrxDDxnUioasnWTxJUmToLddx?cluster=devnet) | 138335 |
| signed voucher + cooperative seal | [`Ggs2jguTGs39Voh9…`](https://explorer.solana.com/tx/Ggs2jguTGs39Voh9NuyGSvdgrbDzKGavmf6hQzZNsAY13BYs7zsYojvkKMrJshnGvM8woTF99vKaEL5snZyrBvw?cluster=devnet) | 513 |
| withdraw refund before terminal distribution | [`24GhJK2iMftMKptQ…`](https://explorer.solana.com/tx/24GhJK2iMftMKptQersrRJr8yT2E9aAQcHFk28jBGzzpmPW5qhByRHXLRuRg3twbcjj9vSHik7Li4gTCLDSFwmVK?cluster=devnet) | 116744 |
| revoke refund delegate | [`5HJvkcyMnVFfUECf…`](https://explorer.solana.com/tx/5HJvkcyMnVFfUECfSKads5Nfw7y1JEZqKJufaPYEcVipDJrs5QgRRWQZ6U9CQREuQwWyW8Fgebi3UmUxAoTRfSpn?cluster=devnet) | 102 |
| zero-refund terminal succeeds without delegate | [`5pqbQ7sHoYCizKtd…`](https://explorer.solana.com/tx/5pqbQ7sHoYCizKtd2jcHt8KQ5kkfcmGM4K3JUXNnu72TQFRwVeVUifaz1Y6QXAkWfpZpmUsYXu2itwDY2N4i5ptB?cluster=devnet) | 105312 |
| approve bounded refund delegate | [`4fr6ZwS3xSRKucsz…`](https://explorer.solana.com/tx/4fr6ZwS3xSRKucszFYut53QZ5jNav9y27n3xi8dNN9Q6LWK5AazBntyKQPdF1jGe7wawUE84YnF7im1RFwAGRnbo?cluster=devnet) | 123 |
| owner + issuer authorize one incarnation | [`3sFJcMgrZpSyywVN…`](https://explorer.solana.com/tx/3sFJcMgrZpSyywVN4xcogeyDtuYP6CF4pyhpXJCE5NKVSYkCVdvFvWP8h5paebYUnYcFTXLwULRMZ7AnYJtKXrtg?cluster=devnet) | 53240 |
| cancel expired never-funded authorization | [`3uorpX2aFkfu5Hxo…`](https://explorer.solana.com/tx/3uorpX2aFkfu5HxoE4MGwqUoaADxn4tewkmo93REHR8w3HEBh4Hh14LVtPvJksvsq55tGLFcV8eAXabLPNhLXimq?cluster=devnet) | 5751 |
| approve bounded refund delegate | [`5B9mmdVNcATbMpgo…`](https://explorer.solana.com/tx/5B9mmdVNcATbMpgonQXcrUU91c7UU1pYo4QKknTu9fUjEU1wMZfL6uByi6JWn29SoCEeZCcJF4ectxpyAnyCJQS8?cluster=devnet) | 123 |
| owner + issuer authorize one incarnation | [`5ySFLFrHxzsxykzq…`](https://explorer.solana.com/tx/5ySFLFrHxzsxykzqCNrBhoGZ3FBauSmP64Ls6oe1d5S2Gcenk5zQVZDJvSy6c4LV3FSzMeZqFnuJQHADCbaGxime?cluster=devnet) | 56240 |
| fresh nonce after cancellation can fund | [`5FkY64KkKFi7AcRt…`](https://explorer.solana.com/tx/5FkY64KkKFi7AcRtBHngQocNfnBp5mFDRwBZv7zBvVBtzT3gGU8RgBrjpHEFgBWfY1jdyMMiTYmNKQ69Jser7qzx?cluster=devnet) | 162335 |

## Not covered / caveats
- pay-kit / pay don't emit `*WithHooks` instructions yet. The harness builds channel instructions directly, so no pay-kit client or merchant was involved.
- `CHNLxBt…` is not byte-verified against `23193ab` (dump `3e94032d…` vs local `538e8b3f…`), but the hook ABI behaved as `23193ab` across every case.
- E2E ran while the deployer was still admin. Governance then moved to Squads (below). No containerized `solana-verify` hash yet (no Docker).
- Leftover SOL in ephemeral test owner keys is not recovered (about 0.1 SOL per run).

## Squads v4 governance on devnet (2026-11-02, ET)
| Item | Value |
|---|---|
| Squads v4 program | `SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf` |
| Multisig | [`9nebqfSd37E3YyodB6iEMz18JjWP3QoydporvedkjDw2`](https://explorer.solana.com/address/9nebqfSd37E3YyodB6iEMz18JjWP3QoydporvedkjDw2?cluster=devnet) |
| Vault PDA (index 0) = **program admin + upgrade authority** | [`2jBifstNwqWdrswEy8chotmVJcwfMRZkV7miqjR86qdx`](https://explorer.solana.com/address/2jBifstNwqWdrswEy8chotmVJcwfMRZkV7miqjR86qdx?cluster=devnet) |
| Threshold | **2 of 3**, time lock 0, config authority none (autonomous), all members with full permissions |
| Members | `BcevhuQmZd4zN56tx988MRbmaGu6n1tU7UWrwkwgVata` (deployer) · `2GTTWuJspGVmMF9iUzBxxiHgh921zom5sJMpqecUkDG5` (user's devnet wallet; pubkey only) · `CFsfxsmtSfWTwGhVnCyFwA1AjM3Ld6yQKPZukvVuSyCw` (throwaway, box-held) |
| Script | `scripts/squads-devnet/run.js` (`@sqds/multisig` 2.x). Idempotent; logs pubkeys and signatures only. |

| Step | Signature |
|---|---|
| multisigCreateV2 (2-of-3) | `g2VVwSBxLqqq3cS3XAkFLccJ7FGzuaaULmXySCBua15QFFU3fU6nAGEKQoWiK78mMwVfKroU6WbxJKfta2rmaK3` |
| propose_admin(deployer → vault), direct | `2uRHRoQdFdVoxECyHpwBrNdVy7nGTAANYtCJ4sH2Y4MyABxaz6vgHzq12uwrFzVj6AYP5twxkbbYHfbsU5pBmz8M` |
| accept_admin vault tx #1: create / proposal / approve deployer | `Tw6rrM3iKQhxC1qrKBs6ufsMsXSZcAUAgb9yM2SwUWbW8DBgfibzU37TLKKNsFkcpRNbnrd77V3qoJAmTYS8seG` / `3k4fCrLuCNMqEWKuaGap7N4i3widYL7HE1UCfXJWGSSn6TzN9QqDsNS3tkYZpvG8NFKQXpBw9UwYJnj6S2z1LVpX` / `5MDy6ooA9twGsEGiLoBeL7rPpoza4ysMSL1Gzyd4ykmRAu7KDK22oqL8ysWDgX2SMU4wvJybDXjkso1pN5xeoa17` |
| Execute with 1/2 approvals | **rejected** client-side by Squads ("Invalid proposal status"; proposal not yet Approved) |
| accept_admin: approve member3 / **execute** | `5MqUwC6LDaCQvGj8jC1kYuXcTZ8Kr94GE5nRZkACq8PjU3CooXcjGy45S2L5LT4PgMgW2Wk2u6m9sWpfho9qw7wA` / `3hfptzArs17eEXcQRYoiM4NdiBnayyNzhZRydTmbzuNAmWdyfYoasPtyQJwo9d193beEAxsTcq7368vPG2CrQr4v` |
| set-upgrade-authority → vault (slot 517916594) | `62uJwroy4pwNkEh6m9XXbZebZBprgfaAf6j4BtPTZoK2tbFwZvryU2LiKe6u5N7C53463GwsGW1hH8zk9Z9nAk4` |
| set_paused(1) vault tx #2: create / proposal / approve ×2 / **execute** | `Y1BK4cs94oKSA44aYTDMsKMpWKXh5FeBUo2qSpzP5W9re1qmpdvBN8mG4pbKHDcncJw8MUMUKWD35YCDQTdZLK7` / `Gg44q1wZwJHUtcv96Npub5XAZ98z7qxjpyUXp1c9JEEv9gXKJnuJV8LpJvNchbSFPTBhoS8zncoSmtcVs3wudbA` / `wVmdT4GNBTN7uHYc8kiHLa91cA2DugeNCHR9dNt2e7BifhTdssDnDaWs8osvwjRHk15J8RGKSzKuxgu5AXUpv7q`, `58wJqKFydWCoNZ82j7YpNmPg8jRU6boyZKPzULVHySPARC6xqWGxKq5ZgyzrD62j2PHadxmjn7SU7gnzH1ZuLs4M` / `5Ma9rEaGoEcKekCNwmmJhSeB1sVNTc8LV62aNnwDSK4gddWr9PgvWcsTdqFd1oQR28rohTHhn3tKnWSzKvzuHtdQ` |
| **Direct set_paused by old deployer key: failed on-chain** with `Custom(0x4d420002)` UNAUTHORIZED | `2bn68M3sbDkibgRGgKLYBnAhZQbMJR2CAx3PcKHLSzBc92sb2Dyyf3yKuc5GeutN9Kg7dCNUwgW5CpRchD5yuPpf` |
| set_paused(0) vault tx #3: create / proposal / approve ×2 / **execute** | `3jmyZRTEz9nauWQesJW9F6PSGwg9Em6sE5C5ZCeU9vAqak1DDMB9ENqeVjrYakVFDM2ALa1FhGvYroDSjb44hm5T` / `Vpanf7BezwJnvVdgMFohnbeoXoFSxyCw6toeGVfNcLzJSZ9UutyPnBZkoyPmaZez3SLMLd18J4wRUyJDUu4qyhZ` / `46RhPjPya8CZ3AyaibNouavgLF4jwoSuqbJoq9cGG9RUL2JgX2CBKQm2RxDpEsSawdSuZUNfKCNT58DUnRvKtMGQ`, `3KKBNok4QnTjovLYv43KUEmX1GbZWhPmAqEvS1Woj5G4dabq6o74GHGxoAjnzY9JPns77eaQv8TvS4HM4d2VZFhc` / `2NLvxMjyuKUCyifvzCAv8CHGeTGHPw23MDz2D4ZKr9go11aWbE2K5N4FKwT8mg6PLbP4rgpDzpsrJwdXJJVxsYYg` |

Final state, verified: `solana program show` authority = vault. Config: admin = vault, pending = 0, paused = false. Cost: 0.01450664 SOL from the deployer, including the authority change.

Consequences and caveats:
- The deployer key is now an ordinary member. It can no longer upgrade the program or pause it on its own.
- ~~Ledger `initialize` required issuer == upgrade authority.~~ Fixed by the v2 upgrade below (`set_issuer`).
- On devnet, two of the three member keys sit on the same box (deployer and throwaway). That proves the mechanics only, not key separation. **Mainnet needs members you choose, ideally hardware wallets on separate devices**, plus a reviewed threshold and time lock. A config authority of none means membership changes need a multisig config transaction.
- The program has no Squads-specific code. The vault simply signs through `vault_transaction_execute`.

## v2 upgrade: separate issuer (2026-11-02, ET)
The v2 program adds `set_issuer` (tag 9, admin only) and config v2 (99 B, `issuer` field). Ledger `initialize` now takes the config PDA as its 8th account. If `Config.issuer` is set, the issuer must equal it. Otherwise the legacy upgrade-authority rule applies. `set_issuer` migrates a v1 (67 B) config in place: realloc to 99 B, with the payer topping up rent. All other instructions read both layouts.

| Item | Value |
|---|---|
| Build | `scripts/build-verifiable.sh devnet`, 87,240 B, sha256 `4af58a20ee217e920cd050f311f8f99b86ffedd075a07f482f7aad8f41f7c62e`. Under 96,000 B, so no extend was needed. |
| Buffer | `FkASuGFBsUCUCnGQGqyJeYjpdG9JUo91RhHx1pAWaMNP`. Written by the deployer, and the dump sha256 matched the build. `set-buffer-authority` → vault: `zLyFksZ9WHUt4eEPVDuR47edgCCctaTqZ41JviA1omJn3bhQRepJYJERHv2y5bEuUUoKCdoGUu6e8b89ZsKYE9k`. The buffer was closed by the upgrade, with rent spilled to the deployer. |
| Upgrade slot | **517920049** |
| On-chain hash | dump, first 87,240 B: **`4af58a20…7c62e`, match**. The rest is zero padding. |
| Issuer | `8CBpS8SE5SpasbvS6LL8Y3JqWYDdAmVL9MhwPHCGuX1v` (new throwaway, box-held, never printed) |
| Config after | 99 B, version 2, admin = vault, issuer = `8CBpS8SE…`, paused = false |

| Step (Squads 2-of-3: deployer + throwaway approve) | Signature |
|---|---|
| program upgrade (buffer FkASuG…): vault tx #4 create | `2ehXQQ6vLSCQuyfczYs6NMNQo3nmkLK5ob2YxFikNPbn1gDQVRzDEwSWG6KoWBVszZ3nYNavn4hRueUnWY46BhVJ` |
| program upgrade (buffer FkASuG…): proposal create | `5udGLV88oyBMhwSynxF31RLBZp3CtW83TaLLQgHtVaE8iiJZ7GHQQ787gKP1hNrEEF93wBSWjpEdrbzCsfnyXYXM` |
| program upgrade (buffer FkASuG…): approve (deployer) | `yhztA4U98xuox8HCevdUM5tqfShtxAu3WW3SrnUp5oYjskBWYUTEDUnymgTpHnch8oHUsyxwBP3gfqvabMKycSP` |
| program upgrade (buffer FkASuG…): approve (member3) | `5FCa4UpRkFn7LFYJnWxQtZETEXmtefEE21Nx1syXcA4X8td4EjVefbMpCYNf9m14U5GgHJSrzCjRvXaEi2ahgDZc` |
| program upgrade (buffer FkASuG…): execute | `beTCRxyCJigiye7u9ocomv7Hc9FxD1Vf72u32dHDPNp3arEnpWUAghkzQNGs9GTVSayPMtzs3VXe9CmuCtQC8FR` |
| fund vault 0.002 SOL (pays config v1->v2 migration rent) | `43SJ8jPC3jEarK7p4P8tiWW8xRHj8NT3kmK9BrrLbmEw8dHb2Wt1pv2qdkW7efsyKhusAJWdn8AuvSiPGHf9rSKA` |
| set_issuer (migrates config v1->v2): vault tx #5 create | `fDH7xcyaAko4v2Wr82K297LjM5cca1LWZ7sMhkcE7qZ3QZZcUGmLDTTQq1qpmLr9CRNg1hHTuWquoBdP2A3TanX` |
| set_issuer (migrates config v1->v2): proposal create | `5tGpkGNEkNapJYkk9KExEvFehTY8oEJgiEdWVKuZUuESomLEKR7tD3QdTGCDBvKD52ZcNJok42GMFvhByMqiPEV` |
| set_issuer (migrates config v1->v2): approve (deployer) | `58G4ukNgpK3kCYZ733Reyq89MvioZG7ZBxy97GhT6YEKdL1Td1MknXQnopBDpaYJnjyEdWH4BHyRXJDoWzeG9QfZ` |
| set_issuer (migrates config v1->v2): approve (member3) | `2xbDw7HowNJE3BqFPbeABjTVSvABrSo3FmZrx5zkjXgocmhsJBpYJTyPEyAEgkECEisw9jkyWT2noeeHRYbFDzZ9` |
| set_issuer (migrates config v1->v2): execute | `4vABrtwuY1jrq9G8CiqshWS9whJpSJMxyv6usssBH8r3xo2aQz2sBgD9kzgEFqQY6Gtfvw1MGmGUzjV9Vnzf2cpD` |
| direct set_issuer by old deployer key REJECTED on-chain {"InstructionError":[0,{"Custom":1296171010}]} | `2va9rwtJwML4xqfv8p5THpHSqSkBMbimDCsPXwCzStPNBE3dW29N7gMPYZ7qWU1PT1o1UdXJ5WaGL6RXtqEMMD2c` |

Devnet e2e rerun with the configured issuer (deployer only funds; issuer `8CBpS8SE…` co-signs initialize and authorize): **43 PASS, exit 0**. Evidence: `evidence/devnet-run-2026-11-02-v2-issuer.txt`. It covers the impostor-issuer and missing-signature rejects plus the full hook flow (deposit, authorize, open/pre_fund, topUp, seal, distribute/post_distribute refund, withdraw, zero-refund terminal, expiry, cancel, replay rejects). Governance flows are not repeated in this mode, because the admin is the vault; they are covered by the Squads proposals above. The harness now prepends `SetComputeUnitLimit(400000)` to channel transactions. One Surfpool run hit the 200k default limit during a hooked `open`, because PDA bump searches vary by key set.
