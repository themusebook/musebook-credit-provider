//! Musebook token-backed API credit custody, adapted from the payment-channels example for payment-channel V2 callbacks.
#![no_std]

#[cfg(feature = "client")]
extern crate alloc;

#[cfg(not(target_os = "solana"))]
extern crate std;

use borsh::{BorshDeserialize, BorshSerialize};
use payment_channel_interface::hooks::{
    HookContext, HookDescriptor, HookMeta, HookState, descriptor_commitment,
};
use pinocchio::{AccountView, Address, ProgramResult, error::ProgramError};

#[cfg(not(feature = "no-entrypoint"))]
pinocchio::program_entrypoint!(process_instruction);
#[cfg(target_os = "solana")]
pinocchio::no_allocator!();
#[cfg(target_os = "solana")]
pinocchio::nostd_panic_handler!();

#[cfg(feature = "client")]
pub mod client;
pub mod events;
pub mod governance;
pub use governance::error as governance_error;
mod processor;

pub const LEDGER_SEED: &[u8] = b"musebook-credit";
pub const CALLBACK_ENTRYPOINT: [u8; 8] = *b"musebook";
pub const INITIALIZE: u8 = 0;
pub const DEPOSIT: u8 = 1;
pub const AUTHORIZE: u8 = 2;
pub const CANCEL_UNFUNDED: u8 = 3;
pub const INITIALIZE_CONFIG: u8 = 4;
pub const SET_PAUSED: u8 = 5;
pub const PROPOSE_ADMIN: u8 = 6;
pub const ACCEPT_ADMIN: u8 = 7;
pub const SET_LEDGER_SUSPENDED: u8 = 8;
/// Program-wide governance config PDA: `[CONFIG_SEED]`.
/// The payment-channels program whose callbacks are trusted. Default is the
/// canonical `CHNLxYv…` ID; `devnet-deployment` selects the alternate devnet
/// V2-hooks deployment `CHNLxBt…` (devnet `CHNLxYv…` is an older 66 KB build).
#[cfg(not(feature = "devnet-deployment"))]
pub const CHANNEL_PROGRAM: Address = payment_channel_interface::ID;
#[cfg(feature = "devnet-deployment")]
pub const CHANNEL_PROGRAM: Address =
    solana_address::address!("CHNLxBtXw7VPqd5GMcbtVoEbn4ehyaSoiHj56Ao5sWkJ");
pub const CONFIG_SEED: &[u8] = b"musebook-config";
/// Squads v4 multisig program (same ID on devnet and mainnet-beta). Informational:
/// the program never special-cases it; a Squads vault PDA is simply a signer.
pub const SQUADS_V4: Address =
    solana_address::address!("SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf");
pub const LOADER: Address = solana_address::address!("BPFLoaderUpgradeab1e11111111111111111111111");

/// Both the credit owner and the stored issuer must sign this exact instruction.
/// A nonce is consumed once; there is no active-channel renewal operation.
#[derive(Clone, Copy, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct Authorization {
    pub payee: [u8; 32],
    pub authorized_signer: [u8; 32],
    pub distribution_hash: [u8; 32],
    pub salt: u64,
    pub open_slot: u64,
    pub cap: u64,
    pub nonce: u64,
    pub expires_at_slot: u64,
}

/// Borsh, fixed size, owned exclusively by this example program.
#[derive(Clone, Copy, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct Ledger {
    pub version: u8,
    pub busy: bool,
    pub active: bool,
    /// Admin-set per-ledger stop. Blocks `pre_fund` (new channel funding) for an
    /// already-active authorization; refunds via `post_distribute` still run.
    pub suspended: bool,
    pub owner: [u8; 32],
    pub mint: [u8; 32],
    pub issuer: [u8; 32],
    pub channel: [u8; 32],
    pub authorization: Authorization,
    pub configuration: [u8; 32],
    pub credits: u64,
    pub funded: u64,
    pub refunded: u64,
    pub last_sequence: u64,
}

impl Ledger {
    pub const LEN: usize = 332;
}

/// Global governance state. `admin` may be any signer, including a Squads v4
/// vault PDA (the vault signs through `vault_transaction_execute`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Config {
    pub version: u8,
    pub bump: u8,
    /// When true: `deposit`, `authorize` and every `pre_fund` callback fail.
    /// `post_distribute` (refund restoration) and `cancel_unfunded` stay open
    /// so a pause can never trap user funds.
    pub paused: bool,
    pub admin: [u8; 32],
    /// Two-step handover target; all-zero when none is pending.
    pub pending_admin: [u8; 32],
}

impl Config {
    pub const LEN: usize = 67;
}

pub fn config_address(program: &Address) -> (Address, u8) {
    Address::find_program_address(&[CONFIG_SEED], program)
}

pub fn ledger_address(program: &Address, owner: &Address, mint: &Address) -> (Address, u8) {
    Address::find_program_address(&[LEDGER_SEED, owner.as_ref(), mint.as_ref()], program)
}

pub fn ata_address(owner: &Address, mint: &Address) -> Address {
    Address::find_program_address(
        &[owner.as_ref(), pinocchio_token::ID.as_ref(), mint.as_ref()],
        &pinocchio_associated_token_account::ID,
    )
    .0
}

/// The complete, immutable hook policy: no caller-selected hooks or extra accounts.
pub fn hook_state(program: &Address, owner: &Address, mint: &Address) -> HookState {
    let ledger = ledger_address(program, owner, mint).0;
    let accounts = [
        HookMeta {
            address: ledger,
            writable: 1,
        },
        HookMeta {
            address: ata_address(&ledger, mint),
            writable: 1,
        },
        HookMeta {
            address: ata_address(owner, mint),
            writable: 1,
        },
        HookMeta {
            address: pinocchio_token::ID,
            writable: 0,
        },
    ];
    let descriptor = HookDescriptor {
        program: *program,
        entrypoint: CALLBACK_ENTRYPOINT,
        account_count: accounts.len() as u8,
        static_len: 0,
        static_context: [0; 32],
        accounts,
    };
    let mut state = HookState::EMPTY;
    state.descriptors[0] = descriptor;
    state.descriptors[3] = descriptor;
    state.commitment = descriptor_commitment(&state.descriptors);
    state
}

/// Decode the shared ABI; the processor additionally validates CPI provenance.
pub fn decode_callback(data: &[u8]) -> Result<HookContext, ProgramError> {
    let context = HookContext::decode_callback(data, &CALLBACK_ENTRYPOINT, &[])?;
    if !matches!(context.phase, 0 | 3) {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(context)
}

pub fn process_instruction(
    program: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    if data.starts_with(&CALLBACK_ENTRYPOINT) {
        return processor::callback(program, accounts, data);
    }
    match data.split_first() {
        Some((&INITIALIZE, [])) => processor::initialize(program, accounts),
        Some((&DEPOSIT, bytes)) => processor::deposit(
            program,
            accounts,
            u64::try_from_slice(bytes).map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        Some((&AUTHORIZE, bytes)) => processor::authorize(
            program,
            accounts,
            Authorization::try_from_slice(bytes)
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        ),
        Some((&CANCEL_UNFUNDED, [])) => processor::cancel_unfunded(program, accounts),
        Some((&INITIALIZE_CONFIG, bytes)) => {
            governance::initialize_config(program, accounts, address_arg(bytes)?)
        }
        Some((&SET_PAUSED, [flag])) => governance::set_paused(program, accounts, bool_arg(*flag)?),
        Some((&PROPOSE_ADMIN, bytes)) => {
            governance::propose_admin(program, accounts, address_arg(bytes)?)
        }
        Some((&ACCEPT_ADMIN, [])) => governance::accept_admin(program, accounts),
        Some((&SET_LEDGER_SUSPENDED, [flag])) => {
            governance::set_ledger_suspended(program, accounts, bool_arg(*flag)?)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn address_arg(bytes: &[u8]) -> Result<[u8; 32], ProgramError> {
    bytes
        .try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)
}

fn bool_arg(flag: u8) -> Result<bool, ProgramError> {
    match flag {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
