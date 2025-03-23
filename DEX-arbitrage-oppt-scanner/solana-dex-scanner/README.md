# Solana DEX Arbitrage Scanner

A Rust-based scanner that monitors price differences between Raydium and Orca DEXes on the Solana network to identify arbitrage opportunities.

## Features

- Monitors token pairs across Raydium and Orca DEXes
- Real-time price comparison and arbitrage opportunity detection
- Telegram notifications for profitable opportunities
- Configurable minimum profit margin
- Support for major Solana tokens (SOL, USDC, USDT)

## Prerequisites

- Rust 1.70 or later
- Solana CLI tools (optional)
- Telegram Bot Token

## Setup

1. Clone the repository:
```bash
git clone <repository-url>
cd solana-dex-scanner
```

2. Copy the example environment file and fill in your values:
```bash
cp .env.example .env
```

3. Edit `.env` with your configuration:
- `SOLANA_RPC_URL`: Your Solana RPC endpoint (default: mainnet-beta)
- `TELEGRAM_BOT_TOKEN`: Your Telegram bot token
- `TELEGRAM_CHAT_ID`: Your Telegram chat ID for notifications

4. Build the project:
```bash
cargo build --release
```

## Usage

Run the scanner:
```bash
cargo run --release
```

The scanner will:
1. Connect to the Solana network
2. Monitor token pairs across Raydium and Orca
3. Calculate price differences
4. Send Telegram notifications when profitable opportunities are found

## Configuration

- Adjust `MIN_PROFIT_MARGIN` in `src/main.rs` to change the minimum profit threshold
- Add or modify tokens in the `TOKENS` constant to monitor different pairs
- Modify the polling interval in `monitor_prices` function if needed

## License

MIT 