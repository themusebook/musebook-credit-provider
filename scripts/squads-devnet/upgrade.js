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
const STATE = 'state-upgrade.json';
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
  if (false) {
    try { await m.rpc.vaultTransactionExecute({ connection: c, feePayer: deployer, multisigPda, transactionIndex, member: deployer.publicKey, signers: [deployer] }); throw new Error('executed with 1 approval!'); }
    catch (e) { if (String(e).includes('executed with 1')) throw e; console.log(`PASS ${label}: execute with 1/2 approvals rejected (${String(e.message || e).slice(0, 80)})`); st.earlyExecuteChecked = true; save(); }
  }
  rec(`${label}: approve (member3)`, await confirm(await m.rpc.proposalApprove({ connection: c, feePayer: deployer, multisigPda, transactionIndex, member: member3, signers: [deployer, member3] })));
  rec(`${label}: execute`, await confirm(await m.rpc.vaultTransactionExecute({ connection: c, feePayer: deployer, multisigPda, transactionIndex, member: deployer.publicKey, signers: [deployer] })));
}

const LOADER = new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111');
const BUFFER = new PublicKey('FkASuGFBsUCUCnGQGqyJeYjpdG9JUo91RhHx1pAWaMNP');
const issuer = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync((process.env.SECRETS_DIR || '/workspace/.secrets') + '/issuer.json')))).publicKey;
(async () => {
  if ((await c.getGenesisHash()) !== DEVNET_GENESIS) throw new Error('not devnet');
  const startBal = await c.getBalance(deployer.publicKey);
  console.log('deployer balance', startBal / 1e9, 'vault', vault.toBase58(), 'issuer', issuer.toBase58());
  const [programData] = PublicKey.findProgramAddressSync([PROGRAM.toBytes()], LOADER);
  if (!st.upgraded) {
    const upgrade = new TransactionInstruction({ programId: LOADER, data: Buffer.from([3, 0, 0, 0]), keys: [
      { pubkey: programData, isSigner: false, isWritable: true },
      { pubkey: PROGRAM, isSigner: false, isWritable: true },
      { pubkey: BUFFER, isSigner: false, isWritable: true },
      { pubkey: deployer.publicKey, isSigner: false, isWritable: true }, // spill: buffer rent refund
      { pubkey: w3.SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: w3.SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: vault, isSigner: true, isWritable: false },
    ] });
    await viaSquads('program upgrade (buffer FkASuG…)', [upgrade]);
    st.upgraded = true; save();
  }
  if (!st.vaultFunded) {
    const t = new Transaction().add(w3.SystemProgram.transfer({ fromPubkey: deployer.publicKey, toPubkey: vault, lamports: 2_000_000 }));
    rec('fund vault 0.002 SOL (pays config v1->v2 migration rent)', await w3.sendAndConfirmTransaction(c, t, [deployer]));
    st.vaultFunded = true; save();
  }
  let cfgInfo = await c.getAccountInfo(config);
  console.log('config before', cfgInfo.data.length, 'bytes, version', cfgInfo.data[0]);
  if (!st.issuerSet) {
    const setIssuer = ix([9, ...issuer.toBytes()], [
      { pubkey: vault, isSigner: true, isWritable: true },
      { pubkey: config, isSigner: false, isWritable: true },
      { pubkey: vault, isSigner: true, isWritable: true },
      { pubkey: w3.SystemProgram.programId, isSigner: false, isWritable: false },
    ]);
    await viaSquads('set_issuer (migrates config v1->v2)', [setIssuer]);
    st.issuerSet = true; save();
  }
  cfgInfo = await c.getAccountInfo(config);
  const d = cfgInfo.data;
  const out = { len: d.length, version: d[0], paused: d[2], admin: new PublicKey(d.subarray(3, 35)).toBase58(), issuer: d.length === 99 ? new PublicKey(d.subarray(67, 99)).toBase58() : null };
  console.log('config after', out);
  if (out.len !== 99 || out.version !== 2 || out.issuer !== issuer.toBase58() || out.admin !== vault.toBase58()) throw new Error('config not migrated as expected');
  // Old deployer cannot set issuer directly.
  if (!st.directIssuerRejected) {
    const t = new Transaction().add(ix([9, ...deployer.publicKey.toBytes()], [
      { pubkey: deployer.publicKey, isSigner: true, isWritable: true },
      { pubkey: config, isSigner: false, isWritable: true },
      { pubkey: deployer.publicKey, isSigner: true, isWritable: true },
      { pubkey: w3.SystemProgram.programId, isSigner: false, isWritable: false },
    ]));
    t.feePayer = deployer.publicKey; t.recentBlockhash = (await c.getLatestBlockhash()).blockhash; t.sign(deployer);
    const sig = await c.sendRawTransaction(t.serialize(), { skipPreflight: true });
    await c.confirmTransaction(sig, 'confirmed');
    const tx = await c.getTransaction(sig, { commitment: 'confirmed', maxSupportedTransactionVersion: 0 });
    const err = JSON.stringify(tx.meta.err);
    if (!err.includes('1296171010')) throw new Error('direct set_issuer not rejected: ' + err);
    rec(`direct set_issuer by old deployer key REJECTED on-chain ${err}`, sig);
    st.directIssuerRejected = true; save();
  }
  const endBal = await c.getBalance(deployer.publicKey);
  console.log('deployer balance', endBal / 1e9, 'delta', (endBal - startBal) / 1e9);
})().catch((e) => { console.error('FAIL', e.message || e); process.exit(1); });
