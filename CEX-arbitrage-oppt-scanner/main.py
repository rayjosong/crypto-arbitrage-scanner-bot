import asyncio
import time
import signal
import sys
import ccxt.async_support as ccxt
from datetime import datetime
import requests
import os
from dotenv import load_dotenv
from functools import wraps

# Load environment variables
load_dotenv()

# List of exchanges to scan
exchanges = {
    'binance': ccxt.binance({"enableRateLimit": True}),
    'kraken': ccxt.kraken({"enableRateLimit": True})
}

# Crypto pairs to analyze
# symbols = [
#     'BTC/USDT',
#     'ETH/USDT',
#     'SOL/USDT',
#     'SOL/USDC',
#     # Add other SPL token pairs here if your CEX supports them. Example: 'SOME_SPL_TOKEN/USDT'
# ]

symbols = [
    # "WBTC/USDT",
    "ETH/USDT",
    "ETH/DAI", 
    "SOL/USDT",
    "SOL/USDC"
]

# Constants should be uppercase
TRADING_FEE = 0.001  # 0.1% per trade
MAX_RETRIES = 3
INITIAL_DELAY = 1

# Flag to control the main loop
running = True

# Telegram configuration
TELEGRAM_BOT_TOKEN = os.getenv("TELEGRAM_BOT_TOKEN")
TELEGRAM_CHAT_ID = os.getenv("TELEGRAM_CHAT_ID")

def telegram_retry(max_retries=3, initial_delay=1):
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            retries = 0
            delay = initial_delay
            while retries < max_retries:
                try:
                    return func(*args, **kwargs)
                except requests.exceptions.HTTPError as e:
                    if e.response.status_code == 429:  # Rate limited
                        retries += 1
                        time.sleep(delay)
                        delay *= 2  # Exponential backoff
                    else:
                        raise
            raise Exception(f"Max retries ({max_retries}) exceeded for Telegram API")
        return wrapper
    return decorator

@telegram_retry()
def send_telegram_message(message: str) -> None:
    try:
        url = f"https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage"
        data = {
            "chat_id": TELEGRAM_CHAT_ID,
            "text": message,
            "parse_mode": "Markdown"
        }
        response = requests.post(url, data=data)
        response.raise_for_status()  # Raise exception for 4XX/5XX status codes
    except Exception as e:
        print(f"Error sending Telegram message: {e}")

async def fetch_ticker(exchange: ccxt.Exchange, symbol: str) -> dict | None:
    try:
        ticker = await exchange.fetch_ticker(symbol)
        return {
            'exchange': exchange.name,
            'symbol': symbol,  # Add symbol to the result
            'bid': ticker['bid'] if ticker['bid'] is not None else 0,
            'ask': ticker['ask'] if ticker['ask'] is not None else float('inf'),
            'timestamp': ticker['timestamp']
        }
    except Exception as e:
        print(f"Error fetching {exchange.name} {symbol}: {e}")
        return None

async def scan_arbitrage(symbol: str) -> bool:
    try:
        tasks = [fetch_ticker(ex, symbol) for ex in exchanges.values()]
        results = await asyncio.gather(*tasks)
        results = [r for r in results if r is not None]  # Filter out failed requests

        if not results:
            print(f"No data fetched for {symbol}, retrying...")
            send_telegram_message(f"⚠️ No data fetched for {symbol} from exchanges, retrying...")
            return None

        lowest_ask = min(results, key=lambda x: x['ask'])
        highest_bid = max(results, key=lambda x: x['bid'])

        buy_price = lowest_ask['ask']
        sell_price = highest_bid['bid']
        profit = sell_price - buy_price
        profit_after_fees = profit - (buy_price * TRADING_FEE) - (sell_price * TRADING_FEE)

        profit_percentage = (profit_after_fees / buy_price) * 100

        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

        if profit_percentage > 0:
            print(f"Arbitrage Opportunity Detected! ({symbol})")
            print(f"Buy on {lowest_ask['exchange']} at {buy_price}")
            print(f"Sell on {highest_bid['exchange']} at {sell_price}")
            print(f"Profit: ${profit_after_fees:.2f} ({profit_percentage:.5f}%)")

            message = f"🚨 *ARBITRAGE OPPORTUNITY* 🚨\n" \
                      f"*Time:* {timestamp}\n" \
                      f"*Pair:* {symbol}\n" \
                      f"*Buy:* {lowest_ask['exchange']} at ${buy_price}\n" \
                      f"*Sell:* {highest_bid['exchange']} at ${sell_price}\n" \
                      f"*Profit:* ${profit_after_fees:.2f} ({profit_percentage:.5f}%)"
            send_telegram_message(message)
        else:
            print(f"No profitable opportunities for {symbol} right now.")
            # Optionally send updates even when no opportunity
            if datetime.now().minute % 15 == 0:  # Send update every 15 minutes
                message = f"📊 *Market Update* - {timestamp}\n" \
                          f"No profitable opportunities for {symbol}\n" \
                          f"Best spread: Buy {lowest_ask['exchange']} (${buy_price}) → " \
                          f"Sell {highest_bid['exchange']} (${sell_price})\n" \
                          f"Spread: {profit_percentage:.5f}%"
                send_telegram_message(message)
        return True #return true to indicate the symbol was processed.

    except Exception as e:
        print(f"Error in scan_arbitrage for {symbol}: {e}")
        return False #return false to indicate the symbol had an error.

async def main() -> None:
    global running

    loop = asyncio.get_running_loop()
    loop.add_signal_handler(signal.SIGINT, signal_handler)
    loop.add_signal_handler(signal.SIGTERM, signal_handler)

    send_telegram_message("🤖 Arbitrage Bot Started\n"
                          f"Monitoring {', '.join(symbols)} across {', '.join(exchanges.keys())}")

    try:
        for exchange in exchanges.values():
            await exchange.load_markets()

        while running:
            tasks = [scan_arbitrage(symbol) for symbol in symbols]
            await asyncio.gather(*tasks)
            await asyncio.sleep(5) #Wait 5 seconds between scans.
    except asyncio.CancelledError:
        print("Main task cancelled")
    except Exception as e:
        print(f"Error in main: {e}")
        send_telegram_message(f"❌ Error in main process: {e}")
    finally:
        print("Closing exchange connections...")
        for exchange in exchanges.values():
            try:
                await exchange.close()
            except Exception as e:
                print(f"Error closing {exchange.name}: {e}")
        print("All connections closed")
        send_telegram_message("👋 Bot has shut down")

def signal_handler() -> None:
    global running
    running = False
    print("\nShutting down gracefully... (Press Ctrl+C again to force)")
    send_telegram_message("🛑 Bot shutting down gracefully...")

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nProgram terminated by user")
    except Exception as e:
        print(f"Unhandled exception: {e}")
    finally:
        print("Program exited")