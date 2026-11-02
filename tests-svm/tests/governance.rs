//! SBF-level governance tests: run the real built `.so` in LiteSVM.
//! Build first: `just build` (writes target/deploy/musebook_credit_provider.so).
use litesvm::LiteSVM;
use musebook_credit_provider::{Config, client, config_address, governance_error as e};
use pinocchio::Address;
use solana_instruction::{Instruction, error::InstructionError};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::{Transaction, TransactionError};

const LOADER: &str = "BPFLoaderUpgradeab1e11111111111111111111111";

fn addr(k: &Keypair) -> Address {
    Address::new_from_array(k.pubkey().to_bytes())
}

struct Env {
    svm: LiteSVM,
    program: Address,
    ua: Keypair,
    admin: Keypair,
}

fn so_path() -> std::path::PathBuf {
    std::env::var("CREDIT_PROVIDER_SO")
        .map(Into::into)
        .unwrap_or_else(|_| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../target/deploy/musebook_credit_provider.so")
        })
}

fn setup() -> Env {
    let mut svm = LiteSVM::new();
    let program = Address::new_from_array([7u8; 32]);
    let program_id = program;
    svm.add_program(
        program_id,
        &std::fs::read(so_path()).expect("build the .so first"),
    )
    .unwrap();
    // Give the ProgramData an upgrade authority (LiteSVM deploys immutable).
    let ua = Keypair::new();
    let loader: Address = LOADER.parse().unwrap();
    let pd = Address::find_program_address(&[program_id.as_ref()], &loader).0;
    let mut acc = svm.get_account(&pd).unwrap();
    acc.data[12] = 1;
    acc.data[13..45].copy_from_slice(&ua.pubkey().to_bytes());
    svm.set_account(pd, acc).unwrap();
    let admin = Keypair::new();
    for k in [&ua, &admin] {
        svm.airdrop(&k.pubkey(), 10_000_000_000).unwrap();
    }
    Env {
        svm,
        program,
        ua,
        admin,
    }
}

impl Env {
    fn send(
        &mut self,
        ix: Instruction,
        signers: &[&Keypair],
    ) -> Result<Vec<String>, TransactionError> {
        self.svm.expire_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&signers[0].pubkey()),
            signers,
            self.svm.latest_blockhash(),
        );
        self.svm
            .send_transaction(tx)
            .map(|m| m.logs)
            .map_err(|f| f.err)
    }

    fn config(&self) -> Config {
        use borsh::BorshDeserialize;
        let key = config_address(&self.program).0;
        Config::try_from_slice(&self.svm.get_account(&key).unwrap().data).unwrap()
    }

    fn init(&mut self) {
        let (ua, admin) = (addr(&self.ua), addr(&self.admin));
        let ix = client::initialize_config(&self.program, &ua, &ua, &admin);
        let ua_kp = self.ua.insecure_clone();
        let logs = self.send(ix, &[&ua_kp]).unwrap();
        assert!(
            logs.iter().any(|l| l.starts_with("Program data: ")),
            "{logs:?}"
        );
    }
}

fn custom(code: u32) -> TransactionError {
    TransactionError::InstructionError(0, InstructionError::Custom(code))
}

fn funded(env: &mut Env) -> Keypair {
    let k = Keypair::new();
    env.svm.airdrop(&k.pubkey(), 1_000_000_000).unwrap();
    k
}

#[test]
fn only_upgrade_authority_can_create_config_and_only_once() {
    let mut env = setup();
    let eve = funded(&mut env);
    let ix = client::initialize_config(&env.program, &addr(&eve), &addr(&eve), &addr(&eve));
    assert!(
        env.send(ix, &[&eve]).is_err(),
        "non-upgrade-authority must be rejected"
    );
    env.init();
    let c = env.config();
    assert_eq!(
        (c.version, c.paused, c.admin),
        (1, false, addr(&env.admin).to_bytes())
    );
    let (ua, admin) = (addr(&env.ua), addr(&env.admin));
    let ua_kp = env.ua.insecure_clone();
    let ix = client::initialize_config(&env.program, &ua, &ua, &admin);
    assert_eq!(
        env.send(ix, &[&ua_kp]),
        Err(custom(e::BAD_CONFIG)),
        "re-init must fail"
    );
}

#[test]
fn pause_blocks_deposit_and_authorize_and_only_admin_toggles() {
    let mut env = setup();
    env.init();
    let eve = funded(&mut env);
    let ix = client::set_paused(&env.program, &addr(&eve), true);
    assert_eq!(env.send(ix, &[&eve]), Err(custom(e::UNAUTHORIZED)));
    assert!(!env.config().paused);

    let admin = env.admin.insecure_clone();
    env.send(
        client::set_paused(&env.program, &addr(&admin), true),
        &[&admin],
    )
    .unwrap();
    assert!(env.config().paused);

    // Paused: the gate fires before any ledger/custody checks.
    let owner = funded(&mut env);
    let mint = Address::new_from_array([3; 32]);
    let ix = client::deposit(&env.program, &addr(&owner), &mint, 1);
    assert_eq!(env.send(ix, &[&owner]), Err(custom(e::PAUSED)));
    let auth = musebook_credit_provider::Authorization {
        cap: 1,
        nonce: 1,
        ..Default::default()
    };
    let ix = client::authorize(&env.program, &addr(&owner), &mint, &addr(&owner), auth);
    assert_eq!(env.send(ix, &[&owner]), Err(custom(e::PAUSED)));

    let ix = client::set_paused(&env.program, &addr(&eve), false);
    assert_eq!(
        env.send(ix, &[&eve]),
        Err(custom(e::UNAUTHORIZED)),
        "eve cannot unpause"
    );
    env.send(
        client::set_paused(&env.program, &addr(&admin), false),
        &[&admin],
    )
    .unwrap();
    // Unpaused: the same call now proceeds past the gate (fails later on the
    // missing ledger, with a different error).
    let ix = client::deposit(&env.program, &addr(&owner), &mint, 1);
    let err = env.send(ix, &[&owner]).unwrap_err();
    assert_ne!(err, custom(e::PAUSED));
}

#[test]
fn two_step_admin_transfer_and_unauthorized_rejects() {
    let mut env = setup();
    env.init();
    let admin = env.admin.insecure_clone();
    let eve = funded(&mut env);
    let next = funded(&mut env);
    let p = env.program;

    let ix = client::propose_admin(&p, &addr(&eve), &addr(&eve));
    assert_eq!(env.send(ix, &[&eve]), Err(custom(e::UNAUTHORIZED)));
    let ix = client::accept_admin(&p, &addr(&eve));
    assert_eq!(env.send(ix, &[&eve]), Err(custom(e::NO_PENDING_ADMIN)));

    env.send(
        client::propose_admin(&p, &addr(&admin), &addr(&next)),
        &[&admin],
    )
    .unwrap();
    assert_eq!(env.config().admin, addr(&admin).to_bytes());
    let ix = client::accept_admin(&p, &addr(&eve));
    assert_eq!(env.send(ix, &[&eve]), Err(custom(e::UNAUTHORIZED)));
    env.send(client::accept_admin(&p, &addr(&next)), &[&next])
        .unwrap();
    let c = env.config();
    assert_eq!(
        (c.admin, c.pending_admin),
        (addr(&next).to_bytes(), [0; 32])
    );

    let ix = client::set_paused(&p, &addr(&admin), true);
    assert_eq!(
        env.send(ix, &[&admin]),
        Err(custom(e::UNAUTHORIZED)),
        "old admin revoked"
    );
    env.send(client::set_paused(&p, &addr(&next), true), &[&next])
        .unwrap();
}

#[test]
fn unsigned_admin_and_non_admin_suspend_rejected() {
    let mut env = setup();
    env.init();
    let eve = funded(&mut env);
    let mut ix = client::set_paused(&env.program, &addr(&env.admin), true);
    ix.accounts[0].is_signer = false;
    // Eve pays, admin key present but not signing.
    let tx_ix = ix.clone();
    assert!(env.send(tx_ix, &[&eve]).is_err());
    assert!(!env.config().paused);
    let ledger = Address::new_from_array([5; 32]);
    let ix = client::set_ledger_suspended(&env.program, &addr(&eve), &ledger, true);
    assert_eq!(env.send(ix, &[&eve]), Err(custom(e::UNAUTHORIZED)));
}
