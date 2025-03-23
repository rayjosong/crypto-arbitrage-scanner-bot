use anyhow::Result;
use colored::*;
use dotenv::dotenv;
use std::env;
use std::time::Duration;

mod dex;
mod models;
mod utils;

use dex::{orca, raydium};
use models::token::TOKENS;
use utils::{price, telegram};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    colored::control::set_override(true);

    let rpc_url = env::var("SOLANA_RPC_URL").expect("SOLANA_RPC_URL must be set");
    let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");
    let telegram_chat_id = env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID must be set");
    let min_profit_threshold = env::var("MIN_PROFIT_THRESHOLD")
        .unwrap_or_else(|_| "0.01".to_string())
        .parse::<f64>()
        .expect("MIN_PROFIT_THRESHOLD must be a valid number");

    let client = solana_client::rpc_client::RpcClient::new(rpc_url);

    println!(
        "{} Starting DEX arbitrage scanner...",
        "[INFO]".bright_green()
    );
    println!(
        "{} Minimum profit threshold: {:.2}%",
        "[INFO]".bright_green(),
        min_profit_threshold * 100.0
    );

    loop {
        for token_a in TOKENS.iter() {
            for token_b in TOKENS.iter() {
                if token_a.address == token_b.address {
                    continue;
                }

                match (
                    raydium::get_pool_data(&client, token_a.address, token_b.address).await,
                    orca::get_pool_data(&client, token_a.address, token_b.address).await,
                ) {
                    (Ok(raydium_pool), Ok(orca_pool)) => {
                        let raydium_price = price::calculate_price(&raydium_pool.reserves);
                        let orca_price = price::calculate_price(&orca_pool.reserves);
                        let profit_margin = price::calculate_profit_margin(raydium_price, orca_price);

                        if profit_margin >= min_profit_threshold {
                            let message = format!(
                                "🚨 <b>Arbitrage Opportunity Found!</b>\n\n\
                                Pair: {}/{} ({}/{})\n\
                                Raydium Price: {:.6}\n\
                                Orca Price: {:.6}\n\
                                Profit Margin: {:.2}%\n\n\
                                <b>Pool Details:</b>\n\
                                Raydium:\n\
                                - Liquidity: {:.2} {}\n\
                                - Fee: {:.2}%\n\n\
                                Orca:\n\
                                - Liquidity: {:.2} {}\n\
                                - Fee: {:.2}%",
                                token_a.symbol,
                                token_b.symbol,
                                token_a.address,
                                token_b.address,
                                raydium_price,
                                orca_price,
                                profit_margin * 100.0,
                                raydium_pool.reserves.token_a as f64
                                    / 10f64.powi(raydium_pool.reserves.decimals_a as i32),
                                token_a.symbol,
                                raydium_pool.fee as f64 / 10000.0,
                                orca_pool.reserves.token_a as f64
                                    / 10f64.powi(orca_pool.reserves.decimals_a as i32),
                                token_a.symbol,
                                orca_pool.fee as f64 / 10000.0
                            );

                            if let Err(e) = telegram::send_telegram_message(
                                &telegram_bot_token,
                                &telegram_chat_id,
                                &message,
                            )
                            .await
                            {
                                println!(
                                    "{} Failed to send Telegram message: {}",
                                    "[ERROR]".bright_red(),
                                    e
                                );
                            }
                        }
                    }
                    (Err(e1), Err(e2)) => {
                        println!(
                            "{} Failed to get pool data for {}/{}: Raydium: {}, Orca: {}",
                            "[ERROR]".bright_red(),
                            token_a.symbol,
                            token_b.symbol,
                            e1,
                            e2
                        );
                    }
                    _ => {}
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
