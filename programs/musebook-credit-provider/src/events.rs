//! Structured program events, emitted with `sol_log_data` so indexers can read
//! them from `meta.logMessages` as `Program data: <base64>...` lines.
//!
//! Every event is `[EVENT_TAG, name, ...fields]`; fields are raw 32-byte keys
//! or little-endian integers. The tag lets an indexer skip unrelated data logs.

pub const EVENT_TAG: &[u8] = b"musebook:v1";

pub const CONFIG_INITIALIZED: &[u8] = b"config_initialized";
pub const PAUSED_SET: &[u8] = b"paused_set";
pub const ADMIN_PROPOSED: &[u8] = b"admin_proposed";
pub const ADMIN_ACCEPTED: &[u8] = b"admin_accepted";
pub const LEDGER_SUSPENDED_SET: &[u8] = b"ledger_suspended_set";
pub const DEPOSITED: &[u8] = b"deposited";
pub const AUTHORIZED: &[u8] = b"authorized";
pub const PRE_FUND: &[u8] = b"pre_fund";
pub const POST_DISTRIBUTE: &[u8] = b"post_distribute";
pub const CANCELED: &[u8] = b"cancel_unfunded";

/// `name` and up to six fields.
pub fn emit(name: &[u8], fields: &[&[u8]]) {
    let mut parts: [&[u8]; 8] = [&[]; 8];
    parts[0] = EVENT_TAG;
    parts[1] = name;
    let n = fields.len().min(6);
    parts[2..2 + n].copy_from_slice(&fields[..n]);
    log_data(&parts[..2 + n]);
}

#[cfg(target_os = "solana")]
fn log_data(parts: &[&[u8]]) {
    // SAFETY: the syscall reads `len` (ptr, len) slice headers, which is the
    // in-memory layout of `&[&[u8]]` on SBF.
    unsafe {
        solana_define_syscall::definitions::sol_log_data(
            parts.as_ptr() as *const u8,
            parts.len() as u64,
        )
    }
}

#[cfg(not(target_os = "solana"))]
fn log_data(_parts: &[&[u8]]) {}
