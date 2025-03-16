# DEX Arbitrage Scanner

A Rust-based DEX arbitrage scanner that monitors Uniswap and Sushiswap for price differences and potential arbitrage opportunities.

## Features
- Real-time monitoring of swap events
- Price comparison between Uniswap and Sushiswap
- Telegram notifications for arbitrage opportunities
- Efficient async processing using Tokio

## Setup

1. Install Rust and Cargo
2. Copy `.env.example` to `.env` and fill in your values:
   - WS_ENDPOINT: Websocket endpoint (e.g., from Alchemy)
   - TELEGRAM_BOT_TOKEN: Your Telegram bot token
   - TELEGRAM_CHAT_ID: Your Telegram chat ID

3. Build and run:
```bash
cargo build --release
cargo run --release
```

## Requirements
- Rust 1.75+
- Ethereum node access (via WebSocket)
- Telegram bot token 