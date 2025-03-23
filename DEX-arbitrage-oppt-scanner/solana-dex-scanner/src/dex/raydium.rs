use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use borsh::BorshDeserialize;
use colored::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use crate::models::pool::{PoolInfo, PoolReserves, RaydiumPoolLayout};

pub const PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const POOL_LAYOUT_SIZE: usize = 1440;
pub const POOL_LAYOUT_VERSION: u8 = 4;

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumPoolInfo {
    id: String,
    baseMint: String,
    quoteMint: String,
    lpMint: String,
    baseDecimals: u8,
    quoteDecimals: u8,
    lpDecimals: u8,
    version: u8,
    programId: String,
    authority: String,
    openOrders: String,
    targetOrders: String,
    baseVault: String,
    quoteVault: String,
    withdrawQueue: String,
    lpVault: String,
    marketVersion: u8,
    marketProgramId: String,
    marketId: String,
    marketAuthority: String,
    marketBaseVault: String,
    marketQuoteVault: String,
    marketBids: String,
    marketAsks: String,
    marketEventQueue: String,
    lookupTableAccount: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RaydiumApiResponse {
    data: Vec<RaydiumPoolInfo>,
}

async fn fetch_pool_info(token_a: &Pubkey, token_b: &Pubkey) -> Result<RaydiumPoolInfo> {
    let client = reqwest::Client::new();
    let url = "https://api.raydium.io/v2/amm/pools";
    
    let response = client.get(url).send().await?
        .json::<RaydiumApiResponse>()
        .await?;

    // Find pool with matching token pair
    for pool in response.data {
        if (pool.baseMint == token_a.to_string() && pool.quoteMint == token_b.to_string()) ||
           (pool.baseMint == token_b.to_string() && pool.quoteMint == token_a.to_string()) {
            return Ok(pool);
        }
    }

    Err(anyhow::anyhow!("Pool not found for token pair"))
}

pub async fn find_pool(
    client: &RpcClient,
    token_a: Pubkey,
    token_b: Pubkey,
) -> Result<Pubkey> {
    let pool_info = fetch_pool_info(&token_a, &token_b).await?;
    let pool_address = Pubkey::from_str(&pool_info.id)?;

    println!("{} Looking for Raydium pool: {}", "[DEBUG]".bright_cyan(), pool_address);
    println!("{} Token A: {}", "[DEBUG]".bright_cyan(), token_a);
    println!("{} Token B: {}", "[DEBUG]".bright_cyan(), token_b);

    // Verify pool exists
    match client.get_account(&pool_address) {
        Ok(_) => {
            println!("{} Found Raydium pool at {}", "[SUCCESS]".bright_green(), pool_address);
            Ok(pool_address)
        }
        Err(e) => {
            println!("{} Failed to find Raydium pool: {}", "[ERROR]".bright_red(), e);
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

    let pool_layout: RaydiumPoolLayout = BorshDeserialize::try_from_slice(&account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize pool data: {}", e))?;

    if pool_layout.version != POOL_LAYOUT_VERSION {
        return Err(anyhow::anyhow!("Unsupported pool version"));
    }

    if !pool_layout.is_initialized {
        return Err(anyhow::anyhow!("Pool not initialized"));
    }

    // Get token decimals from pool info
    let pool_info = fetch_pool_info(&token_a, &token_b).await?;
    let (decimals_a, decimals_b) = if pool_info.baseMint == token_a.to_string() {
        (pool_info.baseDecimals, pool_info.quoteDecimals)
    } else {
        (pool_info.quoteDecimals, pool_info.baseDecimals)
    };

    Ok(PoolInfo {
        reserves: PoolReserves {
            token_a: pool_layout.token_a_reserve,
            token_b: pool_layout.token_b_reserve,
            decimals_a,
            decimals_b,
        },
        fee: pool_layout.fee,
    })
} 