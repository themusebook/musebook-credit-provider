//! Admin authority, emergency pause and per-ledger suspension.
//!
//! The admin is a plain signer key. To put the program under a Squads v4
//! multisig, set the admin to the multisig's *vault PDA* (not the multisig
//! account): Squads signs for the vault PDA when a vault transaction executes.
//! Handover is two-step (`propose_admin` then `accept_admin` signed by the new
//! key), so a typo cannot brick governance.
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::{
    AccountView, Address, ProgramResult, Resize,
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};

use crate::{CONFIG_SEED, Config, Ledger, config_address, events, ledger_address};

/// Custom error codes (0x4d42xx, "MB").
pub mod error {
    pub const PAUSED: u32 = 0x4d42_0001;
    pub const UNAUTHORIZED: u32 = 0x4d42_0002;
    pub const BAD_CONFIG: u32 = 0x4d42_0003;
    pub const NO_PENDING_ADMIN: u32 = 0x4d42_0004;
    pub const SUSPENDED: u32 = 0x4d42_0005;
    pub const INVALID_ADMIN: u32 = 0x4d42_0006;
}

fn err(code: u32) -> ProgramError {
    ProgramError::Custom(code)
}

/// Load and validate the config PDA. Accepts v1 (67 B) and v2 (99 B) layouts.
pub fn load_config(program: &Address, account: &AccountView) -> Result<Config, ProgramError> {
    if !account.owned_by(program) {
        return Err(err(error::BAD_CONFIG));
    }
    let config = decode_config(&account.try_borrow()?)?;
    let (expected, bump) = config_address(program);
    if config.bump != bump || *account.address() != expected {
        return Err(err(error::BAD_CONFIG));
    }
    Ok(config)
}

/// Pure layout decoder: v1 = 67 B with version 1, v2 = 99 B with version 2.
pub fn decode_config(data: &[u8]) -> Result<Config, ProgramError> {
    let version = match data.len() {
        Config::LEN_V1 => 1,
        Config::LEN => 2,
        _ => return Err(err(error::BAD_CONFIG)),
    };
    let mut buf = [0u8; Config::LEN];
    buf[..data.len()].copy_from_slice(data);
    let config = Config::try_from_slice(&buf).map_err(|_| err(error::BAD_CONFIG))?;
    if config.version != version {
        return Err(err(error::BAD_CONFIG));
    }
    Ok(config)
}

/// Write exactly the account's current layout (a v1 account keeps 67 bytes).
fn store_config(account: &mut AccountView, config: &Config) -> ProgramResult {
    let mut buf = [0u8; Config::LEN];
    config
        .serialize(&mut &mut buf[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let mut data = account.try_borrow_mut()?;
    let n = data.len();
    data.copy_from_slice(&buf[..n]);
    Ok(())
}

/// Issuer rule for ledger `initialize`: the explicit config issuer if set.
pub fn configured_issuer(config: &Config) -> Option<[u8; 32]> {
    (config.issuer != [0; 32]).then_some(config.issuer)
}

pub fn apply_set_issuer(config: &mut Config, signer: &[u8; 32], issuer: [u8; 32]) -> ProgramResult {
    if *signer != config.admin {
        return Err(err(error::UNAUTHORIZED));
    }
    config.version = 2;
    config.issuer = issuer;
    Ok(())
}

/// Accounts: admin (s), config (w), payer (s,w; funds the one-time v1->v2
/// migration rent; may be the admin itself, e.g. a funded Squads vault),
/// system program. Data: issuer (32 bytes; all-zero restores the legacy
/// upgrade-authority rule). Affects only ledgers initialized afterwards:
/// each ledger stores its own issuer at `initialize`.
pub fn set_issuer(
    program: &Address,
    accounts: &mut [AccountView],
    issuer: [u8; 32],
) -> ProgramResult {
    let [signer, config_account, payer, system] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !(signer.is_signer() && payer.is_signer() && payer.is_writable()) {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !config_account.is_writable() {
        return Err(err(error::BAD_CONFIG));
    }
    if !(*system.address() == pinocchio_system::ID && system.executable()) {
        return Err(ProgramError::IncorrectProgramId);
    }
    let mut config = load_config(program, config_account)?;
    let previous = config.issuer;
    apply_set_issuer(&mut config, signer.address().as_array(), issuer)?;
    if config_account.data_len() == Config::LEN_V1 {
        let rent = Rent::get()?.try_minimum_balance(Config::LEN)?;
        let pending = rent.saturating_sub(config_account.lamports());
        if pending > 0 {
            pinocchio_system::instructions::Transfer {
                from: payer,
                to: config_account,
                lamports: pending,
            }
            .invoke()?;
        }
        config_account.resize(Config::LEN)?;
    }
    store_config(config_account, &config)?;
    events::emit(
        events::ISSUER_SET,
        &[&previous, &issuer, signer.address().as_ref()],
    );
    Ok(())
}

/// Gate for state that starts new credit exposure.
pub fn require_not_paused(config: &Config) -> ProgramResult {
    if config.paused {
        Err(err(error::PAUSED))
    } else {
        Ok(())
    }
}

// ---- pure state transitions (unit-tested on the host) ----

pub fn apply_set_paused(config: &mut Config, signer: &[u8; 32], paused: bool) -> ProgramResult {
    if *signer != config.admin {
        return Err(err(error::UNAUTHORIZED));
    }
    config.paused = paused;
    Ok(())
}

pub fn apply_propose_admin(
    config: &mut Config,
    signer: &[u8; 32],
    new_admin: [u8; 32],
) -> ProgramResult {
    if *signer != config.admin {
        return Err(err(error::UNAUTHORIZED));
    }
    // All-zero cancels a pending proposal.
    config.pending_admin = new_admin;
    Ok(())
}

pub fn apply_accept_admin(config: &mut Config, signer: &[u8; 32]) -> ProgramResult {
    if config.pending_admin == [0; 32] {
        return Err(err(error::NO_PENDING_ADMIN));
    }
    if *signer != config.pending_admin {
        return Err(err(error::UNAUTHORIZED));
    }
    config.admin = config.pending_admin;
    config.pending_admin = [0; 32];
    Ok(())
}

// ---- instructions ----

/// Accounts: payer (s,w), config (w), program executable, program data,
/// upgrade authority (s), system program. Only the current upgrade authority
/// can create the config, so a front-runner cannot seize governance.
pub fn initialize_config(
    program: &Address,
    accounts: &mut [AccountView],
    admin: [u8; 32],
) -> ProgramResult {
    let [payer, config, executable, program_data, authority, system] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !(payer.is_signer() && payer.is_writable() && config.is_writable()) {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if admin == [0; 32] {
        return Err(err(error::INVALID_ADMIN));
    }
    if !(*system.address() == pinocchio_system::ID && system.executable()) {
        return Err(ProgramError::IncorrectProgramId);
    }
    crate::processor::verify_upgrade_authority(program, executable, program_data, authority)?;
    let (expected, bump) = config_address(program);
    if !(*config.address() == expected
        && config.owned_by(&pinocchio_system::ID)
        && config.data_len() == 0)
    {
        return Err(err(error::BAD_CONFIG));
    }
    let bump_bytes = [bump];
    let seeds = [Seed::from(CONFIG_SEED), Seed::from(&bump_bytes)];
    let rent = Rent::get()?.try_minimum_balance(Config::LEN)?;
    let pending = rent.saturating_sub(config.lamports());
    if pending > 0 {
        pinocchio_system::instructions::Transfer {
            from: payer,
            to: config,
            lamports: pending,
        }
        .invoke()?;
    }
    let signers = [Signer::from(&seeds)];
    pinocchio_system::instructions::Allocate {
        account: config,
        space: Config::LEN as u64,
    }
    .invoke_signed(&signers)?;
    pinocchio_system::instructions::Assign {
        account: config,
        owner: program,
    }
    .invoke_signed(&signers)?;
    let state = Config {
        version: 2,
        bump,
        paused: false,
        admin,
        pending_admin: [0; 32],
        issuer: [0; 32],
    };
    store_config(config, &state)?;
    events::emit(
        events::CONFIG_INITIALIZED,
        &[&admin, authority.address().as_ref()],
    );
    Ok(())
}

fn admin_and_config<'a>(
    program: &Address,
    accounts: &'a mut [AccountView],
) -> Result<(&'a AccountView, &'a mut AccountView, Config), ProgramError> {
    let [signer, config_account, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !signer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !config_account.is_writable() {
        return Err(err(error::BAD_CONFIG));
    }
    let config = load_config(program, config_account)?;
    Ok((signer, config_account, config))
}

/// Accounts: admin (s), config (w).
pub fn set_paused(program: &Address, accounts: &mut [AccountView], paused: bool) -> ProgramResult {
    if accounts.len() != 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let (signer, config_account, mut config) = admin_and_config(program, accounts)?;
    apply_set_paused(&mut config, signer.address().as_array(), paused)?;
    store_config(config_account, &config)?;
    events::emit(
        events::PAUSED_SET,
        &[&[paused as u8], signer.address().as_ref()],
    );
    Ok(())
}

/// Accounts: admin (s), config (w). Data: new admin (32 bytes; zero cancels).
pub fn propose_admin(
    program: &Address,
    accounts: &mut [AccountView],
    new_admin: [u8; 32],
) -> ProgramResult {
    if accounts.len() != 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let (signer, config_account, mut config) = admin_and_config(program, accounts)?;
    apply_propose_admin(&mut config, signer.address().as_array(), new_admin)?;
    store_config(config_account, &config)?;
    events::emit(
        events::ADMIN_PROPOSED,
        &[signer.address().as_ref(), &new_admin],
    );
    Ok(())
}

/// Accounts: pending admin (s), config (w).
pub fn accept_admin(program: &Address, accounts: &mut [AccountView]) -> ProgramResult {
    if accounts.len() != 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let (signer, config_account, mut config) = admin_and_config(program, accounts)?;
    let previous = config.admin;
    apply_accept_admin(&mut config, signer.address().as_array())?;
    store_config(config_account, &config)?;
    events::emit(
        events::ADMIN_ACCEPTED,
        &[&previous, signer.address().as_ref()],
    );
    Ok(())
}

/// Accounts: admin (s), config, ledger (w). The hook descriptor's four account
/// slots are all used, so `pre_fund` cannot read the global config; this flag
/// is how the admin stops an already-authorized ledger from funding channels.
pub fn set_ledger_suspended(
    program: &Address,
    accounts: &mut [AccountView],
    suspended: bool,
) -> ProgramResult {
    let [signer, config_account, ledger_account] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    if !signer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let config = load_config(program, config_account)?;
    if *signer.address().as_array() != config.admin {
        return Err(err(error::UNAUTHORIZED));
    }
    if !(ledger_account.owned_by(program)
        && ledger_account.is_writable()
        && ledger_account.data_len() == Ledger::LEN)
    {
        return Err(ProgramError::InvalidAccountData);
    }
    let mut ledger = Ledger::try_from_slice(&ledger_account.try_borrow()?)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let owner = Address::new_from_array(ledger.owner);
    let mint = Address::new_from_array(ledger.mint);
    if ledger.version != 1
        || ledger.busy
        || ledger_address(program, &owner, &mint).0 != *ledger_account.address()
    {
        return Err(ProgramError::InvalidAccountData);
    }
    ledger.suspended = suspended;
    ledger
        .serialize(&mut &mut ledger_account.try_borrow_mut()?[..])
        .map_err(|_| ProgramError::InvalidAccountData)?;
    events::emit(
        events::LEDGER_SUSPENDED_SET,
        &[ledger_account.address().as_ref(), &[suspended as u8]],
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: [u8; 32] = [1; 32];
    const B: [u8; 32] = [2; 32];
    const EVE: [u8; 32] = [9; 32];

    fn config() -> Config {
        Config {
            version: 2,
            bump: 255,
            paused: false,
            admin: A,
            pending_admin: [0; 32],
            issuer: [0; 32],
        }
    }

    #[test]
    fn admin_can_pause_and_unpause_and_pause_blocks() {
        let mut c = config();
        apply_set_paused(&mut c, &A, true).unwrap();
        assert_eq!(require_not_paused(&c), Err(err(error::PAUSED)));
        apply_set_paused(&mut c, &A, false).unwrap();
        require_not_paused(&c).unwrap();
    }

    #[test]
    fn non_admin_cannot_pause_or_unpause() {
        let mut c = config();
        assert_eq!(
            apply_set_paused(&mut c, &EVE, true),
            Err(err(error::UNAUTHORIZED))
        );
        assert!(!c.paused);
        c.paused = true;
        assert_eq!(
            apply_set_paused(&mut c, &EVE, false),
            Err(err(error::UNAUTHORIZED))
        );
        assert!(c.paused);
    }

    #[test]
    fn two_step_authority_transfer() {
        let mut c = config();
        apply_propose_admin(&mut c, &A, B).unwrap();
        assert_eq!(c.admin, A, "proposal alone must not move authority");
        assert_eq!(
            apply_accept_admin(&mut c, &EVE),
            Err(err(error::UNAUTHORIZED))
        );
        apply_accept_admin(&mut c, &B).unwrap();
        assert_eq!((c.admin, c.pending_admin), (B, [0; 32]));
        // Old admin lost control.
        assert_eq!(
            apply_set_paused(&mut c, &A, true),
            Err(err(error::UNAUTHORIZED))
        );
        apply_set_paused(&mut c, &B, true).unwrap();
    }

    #[test]
    fn unauthorized_propose_and_stale_accept_rejected() {
        let mut c = config();
        assert_eq!(
            apply_propose_admin(&mut c, &EVE, EVE),
            Err(err(error::UNAUTHORIZED))
        );
        assert_eq!(
            apply_accept_admin(&mut c, &EVE),
            Err(err(error::NO_PENDING_ADMIN))
        );
        apply_propose_admin(&mut c, &A, B).unwrap();
        apply_propose_admin(&mut c, &A, [0; 32]).unwrap(); // cancel
        assert_eq!(
            apply_accept_admin(&mut c, &B),
            Err(err(error::NO_PENDING_ADMIN))
        );
        assert_eq!(c.admin, A);
    }

    #[test]
    fn config_layout_is_fixed() {
        let mut bytes = alloc_vec();
        config().serialize(&mut bytes).unwrap();
        assert_eq!(bytes.len(), Config::LEN);
    }

    #[test]
    fn v1_layout_decodes_and_set_issuer_migrates() {
        let mut v1 = config();
        v1.version = 1;
        let mut bytes = alloc_vec();
        v1.serialize(&mut bytes).unwrap();
        let decoded = decode_config(&bytes[..Config::LEN_V1]).unwrap();
        assert_eq!(
            (decoded.version, decoded.admin, decoded.issuer),
            (1, A, [0; 32])
        );
        assert_eq!(configured_issuer(&decoded), None);
        let mut c = decoded;
        assert_eq!(
            apply_set_issuer(&mut c, &EVE, B),
            Err(err(error::UNAUTHORIZED))
        );
        assert_eq!(c.version, 1);
        apply_set_issuer(&mut c, &A, B).unwrap();
        assert_eq!((c.version, configured_issuer(&c)), (2, Some(B)));
        let mut v2 = alloc_vec();
        c.serialize(&mut v2).unwrap();
        assert_eq!(decode_config(&v2).unwrap(), c);
        // Version/length mismatches are rejected.
        assert!(decode_config(&v2[..Config::LEN_V1]).is_err());
        assert!(decode_config(&bytes).is_err());
        assert!(decode_config(&[0u8; 10]).is_err());
    }

    fn alloc_vec() -> std::vec::Vec<u8> {
        std::vec::Vec::new()
    }
}
