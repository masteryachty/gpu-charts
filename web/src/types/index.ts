// Market data types
export interface MarketData {
  symbol: string;
  price: number;
  change: number;
  changePercent: number;
  volume: number;
  timestamp: number;
}

// Chart configuration
export interface ChartConfig {
  symbol: string;
  timeframe: string;
  startTime: number;
  endTime: number;
  indicators: string[];
}

// WASM integration types
export interface WasmModule {
  memory: WebAssembly.Memory;
  // Add other WASM exports as needed
}

// Application state (matches Rust StoreState)
export interface StoreState {
  currentSymbol: string;
  chartConfig: ChartConfig;
  marketData: Record<string, MarketData>;
  isConnected: boolean;
  user?: User;
}

// Keep AppState as alias for backward compatibility
export type AppState = StoreState;

export interface User {
  id: string;
  name: string;
  email: string;
  plan: UserPlan;
}

export enum UserPlan {
  Free = 'free',
  Pro = 'pro', 
  Enterprise = 'enterprise'
}

// Navigation
export interface NavItem {
  label: string;
  href: string;
  icon?: React.ComponentType;
}

// Performance metrics
export interface PerformanceMetrics {
  fps: number;
  frameTime: number;
  latency: number;
}

// Constants (matching Rust implementation)
export const MAX_TIME_RANGE_SECONDS = 86400 * 30; // 30 days
export const MIN_TIME_RANGE_SECONDS = 60; // 1 minute
export const VALID_TIMEFRAMES = ['1m', '5m', '15m', '1h', '4h', '1d'] as const;
export const VALID_COLUMNS = ['time', 'best_bid', 'best_ask', 'price', 'volume', 'side'] as const;

// Validation types
export interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
}

// Data fetch parameters
export interface DataFetchParams {
  symbol: string;
  startTime: number;
  endTime: number;
  columns: string[];
}

// Store validation and serialization utilities
export function validateStoreState(state: StoreState): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Validate current symbol
  if (!state.currentSymbol) {
    errors.push('Current symbol cannot be empty');
  }

  // Validate chart config
  const configValidation = validateChartConfig(state.chartConfig);
  errors.push(...configValidation.errors);
  warnings.push(...configValidation.warnings);

  // Check consistency
  if (state.currentSymbol !== state.chartConfig.symbol) {
    warnings.push(`Current symbol '${state.currentSymbol}' differs from chart config symbol '${state.chartConfig.symbol}'`);
  }

  return { isValid: errors.length === 0, errors, warnings };
}

export function validateChartConfig(config: ChartConfig): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Validate symbol
  if (!config.symbol) {
    errors.push('Symbol cannot be empty');
  }

  // Validate timeframe
  if (!VALID_TIMEFRAMES.includes(config.timeframe as any)) {
    errors.push(`Invalid timeframe '${config.timeframe}'. Must be one of: ${VALID_TIMEFRAMES.join(', ')}`);
  }

  // Validate time range
  if (config.startTime >= config.endTime) {
    errors.push('Start time must be less than end time');
  } else {
    const timeRange = config.endTime - config.startTime;
    if (timeRange < MIN_TIME_RANGE_SECONDS) {
      errors.push(`Time range too small: ${timeRange} seconds (minimum: ${MIN_TIME_RANGE_SECONDS} seconds)`);
    }
    if (timeRange > MAX_TIME_RANGE_SECONDS) {
      warnings.push(`Time range very large: ${timeRange} seconds (maximum recommended: ${MAX_TIME_RANGE_SECONDS} seconds)`);
    }
  }

  // Validate indicators
  config.indicators.forEach(indicator => {
    if (!indicator) {
      warnings.push('Empty indicator name found');
    }
  });

  return { isValid: errors.length === 0, errors, warnings };
}

export function serializeStoreState(state: StoreState): string {
  return JSON.stringify(state);
}

export function deserializeStoreState(json: string): StoreState {
  return JSON.parse(json);
}

export function extractFetchParams(state: StoreState): DataFetchParams {
  return {
    symbol: state.chartConfig.symbol,
    startTime: state.chartConfig.startTime,
    endTime: state.chartConfig.endTime,
    columns: ['time', 'best_bid'] // Default columns
  };
}