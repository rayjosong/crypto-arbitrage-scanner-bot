import { Contract } from "ethers";
import chalk from "chalk";

export type TokenPair = "WBTC/USDT" | "ETH/USDT" | "ETH/DAI";

export interface TelegramConfig {
  botToken: string;
  chatId: string;
}

export interface DexConfig {
  [key: string]: string | ((addr: string) => Contract);
  "WBTC/USDT": string;
  "ETH/USDT": string;
  "ETH/DAI": string;
  factory: string;
  contract: (addr: string) => Contract;
}

export interface DexConfigs {
  [dexName: string]: DexConfig;
}

export interface PriceData {
  exchange: string;
  symbol: TokenPair;
  bid: number;
  ask: number;
  tradingFee: number;
  timestamp: number;
}

export type LogType = "info" | "success" | "warning" | "error" | "arb" | "swap"; 