# DEX Arbitrage Bot

Monitors token pairs across multiple DEXes for arbitrage opportunities. Built with Bun, Ethers.js, and Telegram notifications.

## Features
- Real-time price monitoring
- Cross-DEX arbitrage detection
- Telegram notifications
- Rate limiting
- Event-driven updates

## Setup
```bash
bun add -d typescript @types/node
bun add ethers axios dotenv chalk
bun tsc --init
```

## Configuration
Requires `.env` file with:
- INFURA_PROJECT_ID
- TELEGRAM_BOT_TOKEN
- TELEGRAM_CHAT_ID

## Supported DEXes
- Uniswap
- Sushiswap

## Token Pairs
- WBTC/USDT
- ETH/USDT
- ETH/DAI

## Usage
```bash
bun run main.ts
```

## Dependencies
- typescript
- ethers
- axios
- dotenv
- chalk 