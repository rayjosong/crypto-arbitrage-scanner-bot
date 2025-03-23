use solana_sdk::pubkey::Pubkey;
use once_cell::sync::Lazy;

#[derive(Debug)]
pub struct TokenInfo {
    pub address: Pubkey,
    pub symbol: &'static str,
    pub decimals: u8,
}

pub static TOKENS: Lazy<Vec<TokenInfo>> = Lazy::new(|| {
    vec![
        TokenInfo {
            address: "So11111111111111111111111111111111111111112".parse().unwrap(),
            symbol: "SOL",
            decimals: 9,
        },
        TokenInfo {
            address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".parse().unwrap(),
            symbol: "USDC",
            decimals: 6,
        },
        TokenInfo {
            address: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".parse().unwrap(),
            symbol: "USDT",
            decimals: 6,
        },
    ]
});

pub fn get_token_info(address: &Pubkey) -> Option<&TokenInfo> {
    TOKENS.iter().find(|t| &t.address == address)
} 