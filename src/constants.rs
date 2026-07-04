pub const MIN_AMOUNT_TO_RAISE: u64 = 3;
pub const SECONDS_TO_DAYS: i64 = 86400;
pub const MAX_CONTRIBUTION_PERCENTAGE: u64 = 10;
pub const PERCENTAGE_SCALER: u64 = 100;

pub const ACCOUNT_STORAGE_OVERHEAD: u64 = 128;
pub const LAMPORTS_PER_BYTE: u64 = 6960; // 3480 lamports/byte-year * 2 (exemption threshold)

pub const fn rent_exempt_lamports(len: u64) -> u64 {
    (ACCOUNT_STORAGE_OVERHEAD + len) * LAMPORTS_PER_BYTE
}
