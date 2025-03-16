// Import required packages
import { ethers } from "ethers";
import axios from "axios";
import dotenv from "dotenv";
import chalk from "chalk";

// Load environment variables
dotenv.config();

// ======== CONFIGURATION ========

// Provider configuration
const PROVIDER_URL = `https://mainnet.infura.io/v3/${process.env.INFURA_PROJECT_ID}`;
const provider = new ethers.JsonRpcProvider(PROVIDER_URL);

// Rate limiting configuration
const RATE_LIMIT_DELAY = 1000; // 1 second between calls
let lastCallTime = 0;

// Telegram configuration
const TELEGRAM_CONFIG = {
  botToken: process.env.TELEGRAM_BOT_TOKEN,
  chatId: process.env.TELEGRAM_CHAT_ID,
};

// Token pairs to monitor
const SYMBOLS = ["WBTC/USDT", "ETH/USDT", "ETH/DAI"];

// ======== CONTRACT DEFINITIONS ========

// Uniswap V2 Factory ABI (for potential fee verification)
const FACTORY_ABI = [
  "function feeTo() external view returns (address)",
  "function getPair(address tokenA, address tokenB) external view returns (address pair)",
];

// Uniswap V2 Pair ABI
const PAIR_ABI = [
  "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
  "function token0() external view returns (address)",
  "function token1() external view returns (address)",
  "event Swap(address indexed sender, uint amount0In, uint amount1In, uint amount0Out, uint amount1Out, address indexed to)",
];

// DEX configurations with pair addresses and factory addresses
const DEXES = {
  uniswap: {
    "WBTC/USDT": "0x004375dff511095cc5a197a54140a24efef3a416",
    "ETH/USDT": "0x0d4a11d5eeaac28ec3f61d100daf4d40471f1852",
    "ETH/DAI": "0xa478c2975ab1ea89e8196811f51a7b7ade33eb11",
    factory: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f", // Uniswap V2 Factory
    contract: (addr) => new ethers.Contract(addr, PAIR_ABI, provider),
  },
  sushiswap: {
    "WBTC/USDT": "0xcebff86a11d4ed077e8c571f19e9f2c8ae88eafc",
    "ETH/USDT": "0x06da0fd433c1a5d7a4faa01111c044910a184553",
    "ETH/DAI": "0xc3d03e4f041fd4cd388c549ee2a29a9e5075882f",
    factory: "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac", // SushiSwap Factory
    contract: (addr) => new ethers.Contract(addr, PAIR_ABI, provider),
  },
};

// ======== HELPER FUNCTIONS ========

/**
 * Sends a message to the configured Telegram chat
 * @param {string} message - The message to send
 */
async function sendTelegramMessage(message) {
  try {
    const url = `https://api.telegram.org/bot${TELEGRAM_CONFIG.botToken}/sendMessage`;
    const data = {
      chat_id: TELEGRAM_CONFIG.chatId,
      text: message,
      parse_mode: "Markdown",
    };

    const response = await axios.post(url, data);
    if (response.status !== 200) {
      console.log(`Failed to send Telegram message: ${response.data}`);
    }
  } catch (error) {
    console.log(`Error sending Telegram message: ${error.message}`);
  }
}

/**
 * Gets the current timestamp in a readable format
 * @returns {string} Formatted timestamp
 */
function getFormattedTimestamp() {
  return new Date().toISOString().replace("T", " ").substring(0, 19);
}

/**
 * Applies rate limiting to API calls
 */
async function applyRateLimit() {
  const now = Date.now();
  const timeSinceLastCall = now - lastCallTime;

  if (timeSinceLastCall < RATE_LIMIT_DELAY) {
    await new Promise((resolve) =>
      setTimeout(resolve, RATE_LIMIT_DELAY - timeSinceLastCall)
    );
  }

  lastCallTime = Date.now();
}

/**
 * Fetches the trading fee for a given DEX and pair
 * @param {string} dexName - Name of the DEX
 * @param {string} pairAddress - Address of the pair contract
 * @returns {number} Fee as a decimal (e.g., 0.003 for 0.3%)
 */
async function fetchTradingFee(dexName, pairAddress) {
  try {
    await applyRateLimit();

    const dex = DEXES[dexName];
    // For Uniswap V2 and SushiSwap, the fee is fixed at 0.3% (0.003)
    // We can verify via factory if needed, but it's typically static
    let fee = 0.003; // Default for V2

    // Optional: Verify fee structure via factory (for advanced use cases)
    const factoryContract = new ethers.Contract(
      dex.factory,
      FACTORY_ABI,
      provider
    );
    const feeTo = await factoryContract.feeTo();
    if (feeTo === ethers.ZeroAddress) {
      // No fee recipient means default 0.3% fee to LPs
      fee = 0.003;
    }

    return fee;
  } catch (error) {
    logMessage(
      "error",
      `Error fetching fee for ${dexName} ${pairAddress}: ${error.message}`
    );
    return 0.003; // Fallback to 0.3% if fetching fails
  }
}

// ======== CORE FUNCTIONALITY ========

/**
 * Fetches price data and fees from a specific DEX for a token pair
 * @param {string} dexName - Name of the DEX
 * @param {string} symbol - Token pair symbol
 * @returns {Object|null} Price data with actual fee or null if error
 */
async function fetchDexPrice(dexName, symbol) {
  try {
    await applyRateLimit();

    const dex = DEXES[dexName];
    const pairAddress = dex[symbol];
    const contract = dex.contract(pairAddress);

    // Fetch reserves and token0 address
    const [reserve0, reserve1] = await contract.getReserves();
    const token0 = await contract.token0();

    // Adjust decimals based on token pair
    let decimals0 = 18; // Default for ETH
    let decimals1 = 6; // Default for USDT
    if (symbol === "ETH/DAI") decimals1 = 18;
    else if (symbol === "WBTC/USDT") decimals0 = 8;

    const reserve0Num = Number(ethers.formatUnits(reserve0, decimals0));
    const reserve1Num = Number(ethers.formatUnits(reserve1, decimals1));

    // Calculate price
    const price = reserve1Num / reserve0Num;
    const bid = price * 0.998;
    const ask = price * 1.002;

    // Fetch actual trading fee
    const tradingFee = await fetchTradingFee(dexName, pairAddress);

    return {
      exchange: dexName,
      symbol,
      bid,
      ask,
      tradingFee, // Include the actual fee
      timestamp: Date.now(),
    };
  } catch (error) {
    logMessage(
      "error",
      `Error fetching ${dexName} ${symbol}: ${error.message}`
    );
    return null;
  }
}

// ======== MAIN APPLICATION ========

const LOG_COLORS = {
  info: chalk.blue,
  success: chalk.green,
  warning: chalk.yellow,
  error: chalk.red,
  arb: chalk.magentaBright,
  swap: chalk.cyan,
};

function logMessage(type, message) {
  const timestamp = chalk.gray(`[${getFormattedTimestamp()}]`);
  const prefix = LOG_COLORS[type](`[${type.toUpperCase()}]`);
  console.log(`${timestamp} ${prefix} ${message}`);
}

/**
 * Main application entry point
 */
async function main() {
  logMessage("info", "Starting DEX Arbitrage Bot with Bun...");
  logMessage("info", `Monitoring pairs: ${SYMBOLS.join(", ")}`);
  logMessage("info", `Supported DEXes: ${Object.keys(DEXES).join(", ")}`);

  await sendTelegramMessage(
    `🤖 DEX Arbitrage Bot Started\nMonitoring ${SYMBOLS.join(
      ", "
    )} across ${Object.keys(DEXES).join(", ")}`
  );

  // Initial scan
  await Promise.all(SYMBOLS.map((symbol) => scanArbitrage(symbol)));

  // Setup event listeners for Swap events
  setupEventListeners();

  // Handle shutdown
  process.on("SIGINT", async () => {
    logMessage("warning", "Shutting down gracefully...");
    await sendTelegramMessage("🛑 DEX Bot shutting down gracefully...");
    process.exit(0);
  });
}

/**
 * Scans for arbitrage opportunities between DEXes for a specific token pair
 * @param {string} symbol - Token pair symbol
 * @param {boolean} triggerEvent - Whether this scan was triggered by an event
 */
async function scanArbitrage(symbol, triggerEvent = false) {
  try {
    const results = await Promise.all(
      Object.keys(DEXES).map((dexName) => fetchDexPrice(dexName, symbol))
    );
    const validResults = results.filter((r) => r !== null);

    if (!validResults.length) {
      logMessage("error", `No data for ${symbol}`);
      await sendTelegramMessage(`⚠️ No data for ${symbol}`);
      return;
    }

    const lowestAsk = validResults.reduce((min, curr) =>
      curr.ask < min.ask ? curr : min
    );
    const highestBid = validResults.reduce((max, curr) =>
      curr.bid > max.bid ? curr : max
    );

    const buyPrice = lowestAsk.ask;
    const sellPrice = highestBid.bid;
    const profit = sellPrice - buyPrice;
    const buyFee = buyPrice * lowestAsk.tradingFee;
    const sellFee = sellPrice * highestBid.tradingFee;
    const profitAfterFees = profit - buyFee - sellFee;
    const profitPercentage = (profitAfterFees / buyPrice) * 100;

    if (profitPercentage > 0) {
      const message = formatArbitrageMessage(
        symbol,
        lowestAsk,
        highestBid,
        buyFee,
        sellFee,
        profitAfterFees,
        profitPercentage
      );
      logMessage("arb", `Opportunity: ${symbol}`);
      logMessage("arb", message);
      await sendTelegramMessage(message);
    } else {
      const message = formatNonProfitableMessage(
        symbol,
        lowestAsk,
        highestBid,
        buyFee,
        sellFee,
        profit,
        profitAfterFees,
        profitPercentage,
        triggerEvent
      );
      logMessage("info", `No opportunity: ${symbol}`);
      logMessage("info", message);
      await sendTelegramMessage(message);
    }
  } catch (error) {
    logMessage(
      "error",
      `Error in scanArbitrage for ${symbol}: ${error.message}`
    );
  }
}

function formatArbitrageMessage(
  symbol,
  lowestAsk,
  highestBid,
  buyFee,
  sellFee,
  profitAfterFees,
  profitPercentage
) {
  return (
    `🚨 *PROFITABLE OPPORTUNITY* 🚨\n` +
    `*Time:* ${getFormattedTimestamp()}\n` +
    `*Pair:* ${symbol}\n` +
    `*Buy:* ${lowestAsk.exchange} at $${lowestAsk.ask.toFixed(
      2
    )} (Fee: $${buyFee.toFixed(2)})\n` +
    `*Sell:* ${highestBid.exchange} at $${highestBid.ask.toFixed(
      2
    )} (Fee: $${sellFee.toFixed(2)})\n` +
    `*Total Fees:* $${(buyFee + sellFee).toFixed(2)}\n` +
    `*Profit After Fees:* $${profitAfterFees.toFixed(
      2
    )} (${profitPercentage.toFixed(2)}%)`
  );
}

function formatNonProfitableMessage(
  symbol,
  lowestAsk,
  highestBid,
  buyFee,
  sellFee,
  profit,
  profitAfterFees,
  profitPercentage,
  isSwap
) {
  const title = isSwap ? `📊 *SWAP DETECTED*` : `📊 *MARKET UPDATE*`;
  return (
    `${title} - ${getFormattedTimestamp()}\n` +
    `*Pair:* ${symbol}\n` +
    `*Best Buy:* ${lowestAsk.exchange} at $${lowestAsk.ask.toFixed(
      2
    )} (Fee: $${buyFee.toFixed(2)})\n` +
    `*Best Sell:* ${highestBid.exchange} at $${highestBid.ask.toFixed(
      2
    )} (Fee: $${sellFee.toFixed(2)})\n` +
    `*Total Fees:* $${(buyFee + sellFee).toFixed(2)}\n` +
    `*Potential Profit Before Fees:* $${profit.toFixed(2)}\n` +
    `*Profit After Fees:* $${profitAfterFees.toFixed(
      2
    )} (${profitPercentage.toFixed(2)}%)`
  );
}

/**
 * Sets up event listeners for swap events on all DEXes
 */
function setupEventListeners() {
  SYMBOLS.forEach((symbol) => {
    Object.entries(DEXES).forEach(([dexName, dex]) => {
      const contract = dex.contract(dex[symbol]);

      contract.on(
        "Swap",
        async (
          sender,
          amount0In,
          amount1In,
          amount0Out,
          amount1Out,
          to,
          event
        ) => {
          const decimals0 = symbol === "WBTC/USDT" ? 8 : 18;
          const decimals1 = symbol === "ETH/DAI" ? 18 : 6;

          const swapDetails = [
            amount0In > 0 &&
              `In: ${ethers.formatUnits(amount0In, decimals0)} ${
                symbol.split("/")[0]
              }`,
            amount1In > 0 &&
              `In: ${ethers.formatUnits(amount1In, decimals1)} ${
                symbol.split("/")[1]
              }`,
            amount0Out > 0 &&
              `Out: ${ethers.formatUnits(amount0Out, decimals0)} ${
                symbol.split("/")[0]
              }`,
            amount1Out > 0 &&
              `Out: ${ethers.formatUnits(amount1Out, decimals1)} ${
                symbol.split("/")[1]
              }`,
          ]
            .filter(Boolean)
            .join(" | ");

          logMessage("swap", `${dexName} ${symbol} - ${swapDetails}`);
          await scanArbitrage(symbol, true);
        }
      );
    });
  });
}

// Start the application
main().catch((error) => {
  console.error(`Unhandled error: ${error.message}`);
  sendTelegramMessage(`❌ Error in bot: ${error.message}`);
});
