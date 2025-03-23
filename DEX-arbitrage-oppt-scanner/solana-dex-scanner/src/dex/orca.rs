use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use borsh::BorshDeserialize;
use colored::*;
use crate::models::pool::{PoolInfo, PoolReserves, OrcaPoolLayout};

pub const PROGRAM_ID: &str = "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP";
pub const POOL_LAYOUT_SIZE: usize = 1440;
pub const POOL_LAYOUT_VERSION: u8 = 1;
pub const POOL_SEED_PREFIX: &[u8] = b"whirlpool";

pub async fn find_pool(
    client: &RpcClient,
    token_a: Pubkey,
    token_b: Pubkey,
) -> Result<Pubkey> {
    // Sort tokens to ensure consistent pool address
    let (token_a, token_b) = if token_a < token_b {
        (token_a, token_b)
    } else {
        (token_b, token_a)
    };

    // Generate pool address using PDA
    let (pool_address, _) = Pubkey::find_program_address(
        &[
            POOL_SEED_PREFIX,
            token_a.as_ref(),
            token_b.as_ref(),
        ],
        &PROGRAM_ID.parse().unwrap(),
    );

    println!("{} Looking for Orca pool: {}", "[DEBUG]".bright_cyan(), pool_address);
    println!("{} Token A: {}", "[DEBUG]".bright_cyan(), token_a);
    println!("{} Token B: {}", "[DEBUG]".bright_cyan(), token_b);

    // Verify pool exists
    match client.get_account(&pool_address) {
        Ok(_) => {
            println!("{} Found Orca pool at {}", "[SUCCESS]".bright_green(), pool_address);
            Ok(pool_address)
        }
        Err(e) => {
            println!("{} Failed to find Orca pool: {}", "[ERROR]".bright_red(), e);
            Err(anyhow::anyhow!("Pool not found: {}", e))
        }
    }
}

pub async fn get_pool_data(
    client: &RpcClient,
    token_a: Pubkey,
    token_b: Pubkey,
) -> Result<PoolInfo> {
    let pool_address = find_pool(client, token_a, token_b).await?;
    let account = client.get_account(&pool_address)
        .map_err(|e| anyhow::anyhow!("Failed to get pool account: {}", e))?;

    if account.data.len() != POOL_LAYOUT_SIZE {
        return Err(anyhow::anyhow!("Invalid pool data size"));
    }

    let pool_layout: OrcaPoolLayout = BorshDeserialize::try_from_slice(&account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize pool data: {}", e))?;

    if pool_layout.version != POOL_LAYOUT_VERSION {
        return Err(anyhow::anyhow!("Unsupported pool version"));
    }

    if !pool_layout.is_initialized {
        return Err(anyhow::anyhow!("Pool not initialized"));
    }

    // Get token decimals
    let token_a_info = crate::models::token::get_token_info(&token_a)
        .ok_or_else(|| anyhow::anyhow!("Token A info not found"))?;
    let token_b_info = crate::models::token::get_token_info(&token_b)
        .ok_or_else(|| anyhow::anyhow!("Token B info not found"))?;

    Ok(PoolInfo {
        reserves: PoolReserves {
            token_a: pool_layout.token_a_reserve,
            token_b: pool_layout.token_b_reserve,
            decimals_a: token_a_info.decimals,
            decimals_b: token_b_info.decimals,
        },
        fee: pool_layout.fee,
    })
} 