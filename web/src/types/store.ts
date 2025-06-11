// Store contract for React-Rust WASM integration
// This file defines the exact data structures that will be synchronized between React and Rust

import { AppState, ChartConfig, MarketData, User } from './index';

/**
 * Complete store state interface for WASM serialization
 * This represents the entire Zustand store state that will be sent to Rust
 */
export interface StoreState {
  // Current active symbol
  currentSymbol: string;
  
  // Chart configuration containing all rendering parameters
  chartConfig: ChartConfig;
  
  // Market data keyed by symbol
  marketData: Record<string, MarketData>;
  
  // Connection status to data server
  isConnected: boolean;
  
  // Optional user information
  user?: User;
}

/**
 * Enhanced chart configuration with validation constraints
 * Matches the ChartConfig but with additional metadata for Rust validation
 */
export interface ValidatedChartConfig {
  symbol: string;           // Must be non-empty, alphanumeric + hyphens
  timeframe: string;        // Enum: '1m', '5m', '15m', '1h', '4h', '1d'
  startTime: number;        // Unix timestamp, must be < endTime
  endTime: number;          // Unix timestamp, must be > startTime
  indicators: string[];     // Array of indicator names
}

/**
 * Store update payload for incremental updates
 * Used when only specific parts of the store change
 */
export interface StoreUpdatePayload {
  type: 'symbol' | 'timeRange' | 'config' | 'marketData' | 'connection';
  data: any;
  timestamp: number;        // When the update occurred
}

/**
 * Data fetching parameters extracted from store state
 * This is what Rust will use to determine if new data needs to be fetched
 */
export interface DataFetchParams {
  symbol: string;
  startTime: number;
  endTime: number;
  columns: string[];        // Which data columns to fetch
}

/**
 * Store validation result
 * Used to communicate validation errors between React and Rust
 */
export interface StoreValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
}

/**
 * Type guard functions for runtime validation
 */
export const isValidStoreState = (obj: any): obj is StoreState => {
  return (
    typeof obj === 'object' &&
    obj !== null &&
    typeof obj.currentSymbol === 'string' &&
    typeof obj.chartConfig === 'object' &&
    typeof obj.marketData === 'object' &&
    typeof obj.isConnected === 'boolean'
  );
};

export const isValidChartConfig = (config: any): config is ValidatedChartConfig => {
  return (
    typeof config === 'object' &&
    config !== null &&
    typeof config.symbol === 'string' &&
    config.symbol.length > 0 &&
    typeof config.timeframe === 'string' &&
    typeof config.startTime === 'number' &&
    typeof config.endTime === 'number' &&
    config.startTime < config.endTime &&
    Array.isArray(config.indicators)
  );
};

/**
 * Constants for validation
 */
export const VALIDATION_CONSTANTS = {
  MAX_TIME_RANGE_SECONDS: 86400 * 30, // 30 days max
  MIN_TIME_RANGE_SECONDS: 60,         // 1 minute min
  VALID_TIMEFRAMES: ['1m', '5m', '15m', '1h', '4h', '1d'] as const,
  VALID_COLUMNS: ['time', 'best_bid', 'best_ask', 'price', 'volume', 'side'] as const,
} as const;

export type ValidTimeframe = typeof VALIDATION_CONSTANTS.VALID_TIMEFRAMES[number];
export type ValidColumn = typeof VALIDATION_CONSTANTS.VALID_COLUMNS[number];

/**
 * Serialization helpers for safe JSON conversion
 */
export const serializeStoreState = (state: StoreState): string => {
  try {
    return JSON.stringify(state);
  } catch (error) {
    throw new Error(`Failed to serialize store state: ${error}`);
  }
};

export const deserializeStoreState = (json: string): StoreState => {
  try {
    const parsed = JSON.parse(json);
    if (!isValidStoreState(parsed)) {
      throw new Error('Invalid store state structure');
    }
    return parsed;
  } catch (error) {
    throw new Error(`Failed to deserialize store state: ${error}`);
  }
};