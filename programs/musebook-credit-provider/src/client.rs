//! Host-side builders; addresses use the same type as the on-chain interface.
use alloc::{vec, vec::Vec};
use borsh::BorshSerialize;
use pinocchio::Address;
use solana_instruction::{AccountMeta, Instruction};

use crate::{
    ACCEPT_ADMIN, AUTHORIZE, Authorization, CANCEL_UNFUNDED, DEPOSIT, INITIALIZE,
    INITIALIZE_CONFIG, LOADER, PROPOSE_ADMIN, SET_LEDGER_SUSPENDED, SET_PAUSED, ata_address,
    config_address, hook_state, ledger_address,
};

fn meta(address: Address, writable: bool, signer: bool) -> AccountMeta {
    AccountMeta {
        pubkey: (*address.as_array()).into(),
        is_signer: signer,
        is_writable: writable,
    }
}

fn instruction(program: &Address, accounts: Vec<AccountMeta>, data: Vec<u8>) -> Instruction {
    Instruction {
        program_id: (*program.as_array()).into(),
        accounts,
        data,
    }
}

pub fn initialize(
    program: &Address,
    owner: &Address,
    mint: &Address,
    issuer: &Address,
) -> Instruction {
    let ledger = ledger_address(program, owner, mint).0;
    let program_data = Address::find_program_address(&[program.as_ref()], &LOADER).0;
    instruction(
        program,
        vec![
            meta(*owner, true, true),
            meta(ledger, true, false),
            meta(*mint, false, false),
            meta(*program, false, false),
            meta(program_data, false, false),
            meta(*issuer, false, true),
            meta(pinocchio_system::ID, false, false),
        ],
        vec![INITIALIZE],
    )
}

pub fn deposit(program: &Address, owner: &Address, mint: &Address, amount: u64) -> Instruction {
    let ledger = ledger_address(program, owner, mint).0;
    let mut data = vec![DEPOSIT];
    data.extend_from_slice(&amount.to_le_bytes());
    instruction(
        program,
        vec![
            meta(*owner, false, true),
            meta(ledger, true, false),
            meta(ata_address(&ledger, mint), true, false),
            meta(ata_address(owner, mint), true, false),
            meta(pinocchio_token::ID, false, false),
            meta(config_address(program).0, false, false),
        ],
        data,
    )
}

pub fn authorize(
    program: &Address,
    owner: &Address,
    mint: &Address,
    issuer: &Address,
    authorization: Authorization,
) -> Instruction {
    let ledger = ledger_address(program, owner, mint).0;
    let mut data = vec![AUTHORIZE];
    authorization.serialize(&mut data).unwrap();
    instruction(
        program,
        vec![
            meta(*owner, false, true),
            meta(*issuer, false, true),
            meta(ledger, true, false),
            meta(ata_address(&ledger, mint), true, false),
            meta(ata_address(owner, mint), true, false),
            meta(pinocchio_token::ID, false, false),
            meta(config_address(program).0, false, false),
        ],
        data,
    )
}

/// Cancel only an authorization that has never funded a channel. Its nonce is
/// not reset; subsequent authorization must use the next nonce.
pub fn cancel_unfunded(
    program: &Address,
    owner: &Address,
    mint: &Address,
    issuer: &Address,
) -> Instruction {
    instruction(
        program,
        vec![
            meta(*owner, false, true),
            meta(*issuer, false, true),
            meta(ledger_address(program, owner, mint).0, true, false),
        ],
        vec![CANCEL_UNFUNDED],
    )
}

/// Pass this sparse configuration to the existing V2 `open` builder.
/// Create the governance config. `upgrade_authority` must sign.
pub fn initialize_config(
    program: &Address,
    payer: &Address,
    upgrade_authority: &Address,
    admin: &Address,
) -> Instruction {
    let program_data = Address::find_program_address(&[program.as_ref()], &LOADER).0;
    let mut data = vec![INITIALIZE_CONFIG];
    data.extend_from_slice(admin.as_ref());
    instruction(
        program,
        vec![
            meta(*payer, true, true),
            meta(config_address(program).0, true, false),
            meta(*program, false, false),
            meta(program_data, false, false),
            meta(*upgrade_authority, false, true),
            meta(pinocchio_system::ID, false, false),
        ],
        data,
    )
}

fn admin_ix(program: &Address, signer: &Address, data: Vec<u8>) -> Instruction {
    instruction(
        program,
        vec![
            meta(*signer, false, true),
            meta(config_address(program).0, true, false),
        ],
        data,
    )
}

pub fn set_paused(program: &Address, admin: &Address, paused: bool) -> Instruction {
    admin_ix(program, admin, vec![SET_PAUSED, paused as u8])
}

/// `new_admin` may be a Squads v4 vault PDA. All-zero cancels.
pub fn propose_admin(program: &Address, admin: &Address, new_admin: &Address) -> Instruction {
    let mut data = vec![PROPOSE_ADMIN];
    data.extend_from_slice(new_admin.as_ref());
    admin_ix(program, admin, data)
}

pub fn accept_admin(program: &Address, new_admin: &Address) -> Instruction {
    admin_ix(program, new_admin, vec![ACCEPT_ADMIN])
}

pub fn set_ledger_suspended(
    program: &Address,
    admin: &Address,
    ledger: &Address,
    suspended: bool,
) -> Instruction {
    instruction(
        program,
        vec![
            meta(*admin, false, true),
            meta(config_address(program).0, false, false),
            meta(*ledger, true, false),
        ],
        vec![SET_LEDGER_SUSPENDED, suspended as u8],
    )
}

pub fn hook_config(program: &Address, owner: &Address, mint: &Address) -> Vec<u8> {
    let state = hook_state(program, owner, mint);
    let mut bytes = Vec::new();
    for d in state.descriptors {
        bytes.push(u8::from(d.enabled()));
        if !d.enabled() {
            continue;
        }
        bytes.extend_from_slice(d.program.as_ref());
        bytes.extend_from_slice(&d.entrypoint);
        bytes.push(d.account_count);
        bytes.push(d.static_len);
        bytes.extend_from_slice(&d.static_context[..d.static_len as usize]);
        for m in &d.accounts[..d.account_count as usize] {
            bytes.extend_from_slice(m.address.as_ref());
            bytes.push(m.writable);
        }
    }
    bytes
}

/// Append to a channel instruction's accounts. `phase` is 0 for open/topUp,
/// 3 for distribute/withdrawPayer. The channel program supplies the channel
/// account itself and makes the scoped capability a signer during CPI.
pub fn hook_tail(
    program: &Address,
    owner: &Address,
    mint: &Address,
    channel: &Address,
    phase: u8,
) -> Vec<AccountMeta> {
    assert!(
        matches!(phase, 0 | 3),
        "only the two configured phases are supported"
    );
    let state = hook_state(program, owner, mint);
    let authority = payment_channel_interface::hooks::hook_authority(
        &crate::CHANNEL_PROGRAM,
        channel,
        &state.commitment,
        payment_channel_interface::hooks::Phase::try_from(phase).unwrap(),
        program,
    )
    .0;
    let mut accounts = vec![meta(*program, false, false), meta(authority, false, false)];
    accounts.extend(
        state.descriptors[phase as usize]
            .accounts
            .iter()
            .map(|m| meta(m.address, m.writable == 1, false)),
    );
    accounts
}
