use anyhow::{Result, Context};
use colored::*;
use ethers::{
    contract::Contract,
    providers::{Provider, Ws, Http},
    types::{Address, U256, H160, BlockNumber},
    prelude::*,
    abi::Abi,
};
use std::sync::Arc;
use teloxide::{prelude::*, types::ParseMode};
use tokio;
use dotenv::dotenv;
use std::env;
use futures::StreamExt;
use chrono::Local;
use once_cell::sync::Lazy;
use std::time::Duration;

const UNISWAP_V2_FACTORY: &str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";
const SUSHISWAP_FACTORY: &str = "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac";
const RETRY_DELAY: Duration = Duration::from_secs(5);
const MIN_PROFIT_MARGIN: f64 = 0.01; // 1%

static FACTORY_ABI: Lazy<Abi> = Lazy::new(|| {
    serde_json::from_slice(include_bytes!("./abis/IUniswapV2Factory.json"))
        .expect("Failed to parse factory ABI")
});

static PAIR_ABI: Lazy<Abi> = Lazy::new(|| {
    serde_json::from_slice(include_bytes!("./abis/IUniswapV2Pair.json"))
        .expect("Failed to parse pair ABI")
});

struct TokenInfo {
    address: Address,
    symbol: &'static str,
}

static TOKENS: Lazy<Vec<TokenInfo>> = Lazy::new(|| {
    vec![
        TokenInfo {
            address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(),
            symbol: "WETH",
        },
        TokenInfo {
            address: "0x6B175474E89094C44Da98b954EedeAC495271d0F".parse().unwrap(),
            symbol: "DAI",
        },
        TokenInfo {
            address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(),
            symbol: "USDC",
        },
        TokenInfo {
            address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".parse().unwrap(),
            symbol: "USDT",
        },
        TokenInfo {
            address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".parse().unwrap(),
            symbol: "WBTC",
        },
    ]
});

#[derive(Debug, EthEvent)]
struct SwapEvent {
    #[ethevent(indexed)]
    token0: H160,
    #[ethevent(indexed)]
    token1: H160,
    amount0_in: U256,
    amount1_in: U256,
    amount0_out: U256,
    amount1_out: U256,
}

#[derive(Debug)]
struct PriceInfo {
    token_a: Address,
    token_b: Address,
    symbol_a: &'static str,
    symbol_b: &'static str,
    price_uni: U256,
    price_sushi: U256,
    profit_margin: f64,
}

fn get_token_symbol(address: &Address) -> &'static str {
    TOKENS.iter()
        .find(|t| &t.address == address)
        .map(|t| t.symbol)
        .unwrap_or("UNKNOWN")
}

async fn calculate_prices(
    provider: Arc<Provider<Http>>,
    token0: H160,
    token1: H160,
    uni_factory: &Contract<Provider<Http>>,
    sushi_factory: &Contract<Provider<Http>>,
) -> Result<PriceInfo> {
    let uni_pair = uni_factory
        .method::<_, Address>("getPair", (token0, token1))?
        .call()
        .await?;

    let sushi_pair = sushi_factory
        .method::<_, Address>("getPair", (token0, token1))?
        .call()
        .await?;

    let uni_reserves: (U256, U256, u32) = Contract::new(
        uni_pair,
        PAIR_ABI.clone(),
        Arc::clone(&provider),
    )
    .method("getReserves", ())?
    .call()
    .await?;

    let sushi_reserves: (U256, U256, u32) = Contract::new(
        sushi_pair,
        PAIR_ABI.clone(),
        Arc::clone(&provider),
    )
    .method("getReserves", ())?
    .call()
    .await?;

    let uni_price = if uni_reserves.0 > U256::zero() {
        (uni_reserves.1 * U256::exp10(18)) / uni_reserves.0
    } else {
        U256::zero()
    };

    let sushi_price = if sushi_reserves.0 > U256::zero() {
        (sushi_reserves.1 * U256::exp10(18)) / sushi_reserves.0
    } else {
        U256::zero()
    };

    let profit_margin = if uni_price > sushi_price && sushi_price > U256::zero() {
        uni_price.as_u128() as f64 / sushi_price.as_u128() as f64 - 1.0
    } else if uni_price > U256::zero() {
        sushi_price.as_u128() as f64 / uni_price.as_u128() as f64 - 1.0
    } else {
        0.0
    };

    Ok(PriceInfo {
        token_a: token0,
        token_b: token1,
        symbol_a: get_token_symbol(&token0),
        symbol_b: get_token_symbol(&token1),
        price_uni: uni_price,
        price_sushi: sushi_price,
        profit_margin,
    })
}

async fn init_telegram() -> Result<Bot> {
    let token = env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN not set")?;
    let bot = Bot::new(token);
    Ok(bot)
}

async fn send_telegram_alert(bot: &Bot, chat_id: i64, message: String) -> Result<()> {
    bot.send_message(ChatId(chat_id), message)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

async fn monitor_pair(
    pair: Contract<Provider<Http>>,
    symbol0: &'static str,
    symbol1: &'static str,
    provider: Arc<Provider<Http>>,
    uni_factory: Contract<Provider<Http>>,
    sushi_factory: Contract<Provider<Http>>,
    bot: Bot,
    chat_id: i64,
) -> Result<()> {
    let event_filter = pair.event::<SwapEvent>();
    let mut stream = event_filter
        .stream()
        .await
        .context("Failed to create event stream")?;

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(event) => {
                let time = Local::now().format("%H:%M:%S").to_string();
                println!("{} {} New swap event detected for {}/{}", 
                    "[INFO]".bright_blue(),
                    time.bright_black(),
                    symbol0,
                    symbol1,
                );

                match calculate_prices(
                    Arc::clone(&provider),
                    event.token0,
                    event.token1,
                    &uni_factory,
                    &sushi_factory,
                ).await {
                    Ok(price_info) => {
                        if price_info.profit_margin > MIN_PROFIT_MARGIN {
                            println!("{} {} Arbitrage opportunity found! {}/{} Profit: {:.2}%", 
                                "[ALERT]".bright_yellow(),
                                time.bright_black(),
                                price_info.symbol_a,
                                price_info.symbol_b,
                                price_info.profit_margin * 100.0
                            );

                            let message = format!(
                                "🚨 <b>Arbitrage Opportunity!</b>\n\n\
                                Pair: <code>{}/{}</code>\n\
                                Uniswap Price: <code>{} {}/{}</code>\n\
                                Sushiswap Price: <code>{} {}/{}</code>\n\
                                Profit Margin: <b>{:.2}%</b>",
                                price_info.symbol_a,
                                price_info.symbol_b,
                                price_info.price_uni,
                                price_info.symbol_b,
                                price_info.symbol_a,
                                price_info.price_sushi,
                                price_info.symbol_b,
                                price_info.symbol_a,
                                price_info.profit_margin * 100.0
                            );
                            
                            if let Err(e) = send_telegram_alert(&bot, chat_id, message).await {
                                println!("{} Failed to send Telegram alert: {}", "[ERROR]".bright_red(), e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} Error calculating prices: {}", "[ERROR]".bright_red(), e);
                        tokio::time::sleep(RETRY_DELAY).await;
                    }
                }
            }
            Err(e) => {
                println!("{} Error processing event: {}", "[ERROR]".bright_red(), e);
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
    Ok(())
}

async fn monitor_swaps(provider: Arc<Provider<Http>>, bot: Bot, chat_id: i64) -> Result<()> {
    println!("{}", "\n=== DEX Arbitrage Scanner ===".bright_green().bold());
    println!("{}", "Initializing contracts...".yellow());

    let uni_factory = Contract::new(
        UNISWAP_V2_FACTORY.parse::<Address>()?,
        FACTORY_ABI.clone(),
        Arc::clone(&provider),
    );

    let sushi_factory = Contract::new(
        SUSHISWAP_FACTORY.parse::<Address>()?,
        FACTORY_ABI.clone(),
        Arc::clone(&provider),
    );

    let mut pairs = Vec::new();
    println!("{}", "Fetching token pairs...".yellow());
    
    for token0 in TOKENS.iter() {
        for token1 in TOKENS.iter() {
            if token0.address >= token1.address { continue; }
            
            let uni_pair = uni_factory
                .method::<_, Address>("getPair", (token0.address, token1.address))?
                .call()
                .await
                .context("Failed to get pair address")?;

            if uni_pair != Address::zero() {
                let pair_contract = Contract::new(
                    uni_pair,
                    PAIR_ABI.clone(),
                    Arc::clone(&provider),
                );
                pairs.push((pair_contract, token0.symbol, token1.symbol));
                print!("{}", ".".bright_blue());
            }
        }
    }
    println!("\n");

    println!("{} {} {}", 
        "Monitoring".bright_green(),
        pairs.len().to_string().bright_yellow().bold(),
        "pairs for arbitrage opportunities...".bright_green()
    );
    println!("{}", "Press Ctrl+C to stop\n".bright_black());

    let mut tasks = Vec::new();
    for (pair, symbol0, symbol1) in pairs {
        let provider = Arc::clone(&provider);
        let uni_factory = uni_factory.clone();
        let sushi_factory = sushi_factory.clone();
        let bot = bot.clone();
        
        let task = tokio::spawn(async move {
            loop {
                if let Err(e) = monitor_pair(
                    pair.clone(),
                    symbol0,
                    symbol1,
                    Arc::clone(&provider),
                    uni_factory.clone(),
                    sushi_factory.clone(),
                    bot.clone(),
                    chat_id,
                ).await {
                    println!("{} Error monitoring {}/{}: {}", 
                        "[ERROR]".bright_red(),
                        symbol0,
                        symbol1,
                        e
                    );
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        });
        tasks.push(task);
    }

    futures::future::join_all(tasks).await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    println!("{}", "\n=== DEX Arbitrage Scanner ===".bright_green().bold());
    println!("{}", "\nLoading configuration...".yellow());
    let rpc_url = env::var("RPC_URL").context("RPC_URL not set")?;
    let chat_id = env::var("TELEGRAM_CHAT_ID")
        .context("TELEGRAM_CHAT_ID not set")?
        .parse::<i64>()
        .context("Invalid TELEGRAM_CHAT_ID")?;
    
    println!("{}", "Connecting to Ethereum network...".yellow());
    let provider = Provider::<Http>::try_from(rpc_url)
        .context("Failed to connect to Ethereum network")?;
    let provider = Arc::new(provider);
    
    println!("{}", "Initializing Telegram bot...".yellow());
    let bot = init_telegram().await?;

    // Prepare initialization message
    let token_list = TOKENS.iter()
        .map(|t| t.symbol)
        .collect::<Vec<_>>()
        .join(", ");
    
    // Log to console
    println!("\n{}", "Initialization Details:".bright_blue().bold());
    println!("{} {}", "DEXes:".bright_yellow(), "Uniswap V2, Sushiswap".bright_white());
    println!("{} {}", "Tokens:".bright_yellow(), token_list.bright_white());
    println!("{} {}%\n", "Min Profit:".bright_yellow(), MIN_PROFIT_MARGIN * 100.0);
    
    // Send to Telegram
    let startup_msg = format!(
        "🤖 <b>DEX Arbitrage Scanner Started</b>\n\n\
        Monitoring:\n\
        • Uniswap V2\n\
        • Sushiswap\n\n\
        Token Pairs:\n\
        <code>{}</code>\n\n\
        Minimum Profit: <b>{}%</b>",
        token_list,
        MIN_PROFIT_MARGIN * 100.0
    );
    
    send_telegram_alert(&bot, chat_id, startup_msg).await?;

    monitor_swaps(provider, bot, chat_id).await?;

    Ok(())
} 