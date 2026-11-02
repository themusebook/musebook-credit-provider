// DEVNET ONLY. Logs public keys and signatures only; never keys or mnemonics.
const fs = require('fs');
const m = require('@sqds/multisig');
const w3 = require('@solana/web3.js');
const { Connection, Keypair, PublicKey, TransactionMessage, TransactionInstruction, Transaction } = w3;
const RPC = 'https://api.devnet.solana.com';
const DEVNET_GENESIS = 'EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG';
const PROGRAM = new PublicKey('97F68uhs5fmvddJxH1LrKytbBXrhx6ZBZwuZAdUdP7QA');
const USER_WALLET = new PublicKey('2GTTWuJspGVmMF9iUzBxxiHgh921zom5sJMpqecUkDG5');
const load = (f) => Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync((process.env.SECRETS_DIR || '/workspace/.secrets') + '/' + f))));
const deployer = load('payer.json'), member3 = load('squads-member3.json'), createKey = load('squads-create.json');
const STATE = 'state.json';
const st = fs.existsSync(STATE) ? JSON.parse(fs.readFileSync(STATE)) : { sigs: [] };
const save = () => fs.writeFileSync(STATE, JSON.stringify(st, null, 2));
const rec = (label, sig) => { st.sigs.push({ label, sig }); save(); console.log(`SIG ${label}: ${sig}`); };
const c = new Connection(RPC, 'confirmed');
const [config] = PublicKey.findProgramAddressSync([Buffer.from('musebook-config')], PROGRAM);
const [multisigPda] = m.getMultisigPda({ createKey: createKey.publicKey });
const [vault] = m.getVaultPda({ multisigPda, index: 0 });
const ix = (data, keys) => new TransactionInstruction({ programId: PROGRAM, keys, data: Buffer.from(data) });
const readConfig = async () => {
  const d = (await c.getAccountInfo(config)).data;
  return { paused: d[2] === 1, admin: new PublicKey(d.subarray(3, 35)).toBase58(), pending: new PublicKey(d.subarray(35, 67)).toBase58() };
};
const confirm = async (sig) => { await c.confirmTransaction(sig, 'confirmed'); return sig; };

async function viaSquads(label, instructions) {
  const ms = await m.accounts.Multisig.fromAccountAddress(c, multisigPda);
  const transactionIndex = BigInt(ms.transactionIndex.toString()) + 1n;
  const { blockhash } = await c.getLatestBlockhash();
  const transactionMessage = new TransactionMessage({ payerKey: vault, recentBlockhash: blockhash, instructions });
  rec(`${label}: vault tx #${transactionIndex} create`, await confirm(await m.rpc.vaultTransactionCreate({ connection: c, feePayer: deployer, multisigPda, transactionIndex, creator: deployer.publicKey, vaultIndex: 0, ephemeralSigners: 0, transactionMessage })));
  rec(`${label}: proposal create`, await confirm(await m.rpc.proposalCreate({ connection: c, feePayer: deployer, multisigPda, transactionIndex, creator: deployer })));
  rec(`${label}: approve (deployer)`, await confirm(await m.rpc.proposalApprove({ connection: c, feePayer: deployer, multisigPda, transactionIndex, member: deployer })));
  // 1 of 2 approvals: execute must fail.
  if (!st.earlyExecuteChecked) {
    try { await m.rpc.vaultTransactionExecute({ connection: c, feePayer: deployer, multisigPda, transactionIndex, member: deployer.publicKey, signers: [deployer] }); throw new Error('executed with 1 approval!'); }
    catch (e) { if (String(e).includes('executed with 1')) throw e; console.log(`PASS ${label}: execute with 1/2 approvals rejected (${String(e.message || e).slice(0, 80)})`); st.earlyExecuteChecked = true; save(); }
  }
  rec(`${label}: approve (member3)`, await confirm(await m.rpc.proposalApprove({ connection: c, feePayer: deployer, multisigPda, transactionIndex, member: member3, signers: [deployer, member3] })));
  rec(`${label}: execute`, await confirm(await m.rpc.vaultTransactionExecute({ connection: c, feePayer: deployer, multisigPda, transactionIndex, member: deployer.publicKey, signers: [deployer] })));
}

(async () => {
  if ((await c.getGenesisHash()) !== DEVNET_GENESIS) throw new Error('not devnet');
  const startBal = await c.getBalance(deployer.publicKey);
  console.log('deployer balance', startBal / 1e9, 'member3', member3.publicKey.toBase58());
  console.log('multisig', multisigPda.toBase58(), 'vault', vault.toBase58());
  Object.assign(st, { multisig: multisigPda.toBase58(), vault: vault.toBase58(), threshold: 2,
    members: [deployer.publicKey.toBase58(), USER_WALLET.toBase58(), member3.publicKey.toBase58()] }); save();
  const all = m.types.Permissions.all();
  if (!(await c.getAccountInfo(multisigPda))) {
    const [pc] = m.getProgramConfigPda({});
    const { treasury } = await m.accounts.ProgramConfig.fromAccountAddress(c, pc);
    rec('multisigCreateV2 (2-of-3)', await confirm(await m.rpc.multisigCreateV2({ connection: c, treasury, createKey, creator: deployer, multisigPda,
      configAuthority: null, threshold: 2, timeLock: 0, rentCollector: null,
      members: [deployer.publicKey, USER_WALLET, member3.publicKey].map((key) => ({ key, permissions: all })) })));
  }
  let cfg = await readConfig();
  if (cfg.admin !== vault.toBase58()) {
    if (cfg.pending !== vault.toBase58()) {
      const t = new Transaction().add(ix([6, ...vault.toBytes()], [{ pubkey: deployer.publicKey, isSigner: true, isWritable: false }, { pubkey: config, isSigner: false, isWritable: true }]));
      rec('propose_admin(deployer -> vault)', await w3.sendAndConfirmTransaction(c, t, [deployer]));
    }
    await viaSquads('accept_admin', [ix([7], [{ pubkey: vault, isSigner: true, isWritable: false }, { pubkey: config, isSigner: false, isWritable: true }])]);
  }
  cfg = await readConfig();
  console.log('config after handover', cfg);
  if (cfg.admin !== vault.toBase58()) throw new Error('admin not vault');
  const adminKeys = [{ pubkey: vault, isSigner: true, isWritable: false }, { pubkey: config, isSigner: false, isWritable: true }];
  if (!st.pausedViaSquads) { await viaSquads('set_paused(1)', [ix([5, 1], adminKeys)]); st.pausedViaSquads = true; save(); }
  console.log('after pause', await readConfig());
  if ((await readConfig()).paused !== true) throw new Error('not paused');
  // Old deployer key tries to unpause directly (while paused) — must fail on-chain.
  if (!st.directRejected) {
    const t = new Transaction().add(ix([5, 1], [{ pubkey: deployer.publicKey, isSigner: true, isWritable: false }, { pubkey: config, isSigner: false, isWritable: true }]));
    t.feePayer = deployer.publicKey; t.recentBlockhash = (await c.getLatestBlockhash()).blockhash; t.sign(deployer);
    const sig = await c.sendRawTransaction(t.serialize(), { skipPreflight: true });
    await c.confirmTransaction(sig, 'confirmed');
    const tx = await c.getTransaction(sig, { commitment: 'confirmed', maxSupportedTransactionVersion: 0 });
    const err = JSON.stringify(tx.meta.err);
    if (!err.includes('1296171010')) throw new Error('direct pause not rejected as UNAUTHORIZED: ' + err);
    rec(`direct pause by old deployer key REJECTED on-chain ${err}`, sig); st.directRejected = true; save();
  }
  if (!st.unpausedViaSquads) { await viaSquads('set_paused(0)', [ix([5, 0], adminKeys)]); st.unpausedViaSquads = true; save(); }
  cfg = await readConfig(); console.log('final config', cfg);
  if (cfg.paused) throw new Error('still paused');
  const endBal = await c.getBalance(deployer.publicKey);
  st.spentLamports = (st.spentLamports || 0) + (startBal - endBal); save();
  console.log('deployer balance', endBal / 1e9, 'spent this run', (startBal - endBal) / 1e9);
})().catch((e) => { console.error('FAIL', e.message || e); process.exit(1); });
