use anchor_lang::prelude::*;

#[account]
pub struct ProtocolState {
    pub bump: u8,
    pub vault_authority_bump: u8,
    pub initialized_at: i64,
    pub registration_open_at: i64,
    pub service_treasury: Pubkey,
    pub usdt_mint: Pubkey,
    pub usdc_mint: Pubkey,
    pub pioneer_count: u16,
    pub real_user_count: u64,
    /// Next globally unique logical service-unit ID. Starts at 1.
    pub next_unit_id: u128,
    /// Global per-virtual-share Pioneer index, scaled by PIONEER_SCALE.
    pub pioneer_index_usdt: u128,
    pub pioneer_index_usdc: u128,
    /// Fractional unassigned Pioneer value not yet transferable as a whole token atom.
    pub pioneer_unassigned_remainder_usdt_scaled: u128,
    pub pioneer_unassigned_remainder_usdc_scaled: u128,
    /// Treasury accounting buckets are intentionally separate for auditability.
    pub lifetime_service_fees_usdt: u128,
    pub lifetime_service_fees_usdc: u128,
    pub lifetime_unallocated_usdt: u128,
    pub lifetime_unallocated_usdc: u128,
    pub lifetime_expired_usdt: u128,
    pub lifetime_expired_usdc: u128,
    pub lifetime_rounding_usdt: u128,
    pub lifetime_rounding_usdc: u128,
    pub lifetime_pioneer_unassigned_usdt: u128,
    pub lifetime_pioneer_unassigned_usdc: u128,
}

impl ProtocolState {
    // 8-byte Anchor discriminator + 364 bytes of serialized fields.
    pub const SPACE: usize = 372;
}

#[account]
pub struct UserState {
    pub bump: u8,
    pub wallet: Pubkey,
    pub referrer: Pubkey,
    pub registered_at: i64,
    pub pioneer_id: u16,
    pub active_until: i64,
    pub grace_until: i64,
    pub qualification_progress_units: u64,
    /// Start of the current partial (<10 units) qualification window.
    pub qualification_window_started_at: i64,
    pub lifetime_service_units: u128,
    pub next_batch_index: u64,
    pub direct_accrued_usdt: u64,
    pub direct_accrued_usdc: u64,
    pub network_claimable_usdt: u64,
    pub network_claimable_usdc: u64,
    pub network_pending_usdt: u64,
    pub network_pending_usdc: u64,
    pub lifetime_claimed_usdt: u128,
    pub lifetime_claimed_usdc: u128,
    pub lifetime_expired_usdt: u128,
    pub lifetime_expired_usdc: u128,
    /// Pioneer checkpoints use the same PIONEER_SCALE as the global indexes.
    pub pioneer_checkpoint_usdt: u128,
    pub pioneer_checkpoint_usdc: u128,
}

impl UserState {
    pub const SPACE: usize = 283;
    pub fn is_technical_root(&self) -> bool { self.wallet == Pubkey::default() }
}

#[account]
pub struct UnitBatch {
    pub bump: u8,
    pub owner: Pubkey,
    pub batch_index: u64,
    pub mint: Pubkey,
    pub units: u64,
    pub first_unit_id: u128,
    pub last_unit_id: u128,
    pub purchased_at: i64,
}

impl UnitBatch {
    pub const SPACE: usize = 8 + 1 + 32 + 8 + 32 + 8 + 16 + 16 + 8;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityStatus { Active, Grace, Inactive }
