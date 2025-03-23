use solana_sdk::pubkey::Pubkey;
use borsh::{BorshSerialize, BorshDeserialize};

#[derive(Debug, Clone, Copy)]
pub struct PoolReserves {
    pub token_a: u64,
    pub token_b: u64,
    pub decimals_a: u8,
    pub decimals_b: u8,
}

#[derive(Debug)]
pub struct PoolInfo {
    pub reserves: PoolReserves,
    pub fee: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct RaydiumPoolLayout {
    pub version: u8,
    pub is_initialized: bool,
    pub nonce: u8,
    pub token_program_id: Pubkey,
    pub token_account_a: Pubkey,
    pub token_account_b: Pubkey,
    pub token_pool: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub fee_account: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
    pub fee: u64,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct OrcaPoolLayout {
    pub version: u8,
    pub is_initialized: bool,
    pub nonce: u8,
    pub token_program_id: Pubkey,
    pub token_account_a: Pubkey,
    pub token_account_b: Pubkey,
    pub token_pool: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub fee_account: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
    pub fee: u64,
    pub tick_spacing: u16,
    pub tick_array_start_index: i32,
    pub tick_array_lower_start_index: i32,
    pub tick_array_upper_start_index: i32,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub tick_current_index: i32,
    pub protocol_fee_rate: u16,
    pub protocol_fee_owner: Pubkey,
} 