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
  selectedMetrics: string[]; // Multiple metrics like ['best_bid', 'best_ask']
  chartType: 'line' | 'candlestick';
  candleTimeframe: number; // in seconds (60, 300, 900, 3600, etc.)
}

// WASM integration types
export interface WasmModule {
  memory: WebAssembly.Memory;
  // Add other WASM exports as needed
}

// Store subscription callback interface
export interface StoreSubscriptionCallbacks {
  onSymbolChange?: (newSymbol: string, oldSymbol: string) => void;
  onTimeRangeChange?: (newRange: { startTime: number; endTime: number }, oldRange: { startTime: number; endTime: number }) => void;
  onTimeframeChange?: (newTimeframe: string, oldTimeframe: string) => void;
  onIndicatorsChange?: (newIndicators: string[], oldIndicators: string[]) => void;
  onMetricsChange?: (newMetrics: string[], oldMetrics: string[]) => void;
  onConnectionChange?: (connected: boolean) => void;
  onMarketDataChange?: (symbol: string, data: MarketData) => void;
  onAnyChange?: (newState: AppState, oldState: AppState) => void;
}

// Application state (matches Rust StoreState)
export interface StoreState {
  currentSymbol: string;
  chartConfig: ChartConfig;
  marketData: Record<string, MarketData>;
  isConnected: boolean;
  user?: User;
  // Subscription management for testing
  _subscriptions?: Map<string, StoreSubscriptionCallbacks>;
  _lastState?: StoreState | null;
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

// Import constants from centralized configuration
import { 
  STORE_CONSTANTS, 
  VALID_TIMEFRAMES, 
  VALID_COLUMNS, 
  validateSymbol, 
  validateTimeRange,
  isValidTimeframe 
} from '../config/store-constants';

// Re-export commonly used constants for backward compatibility
export const MAX_TIME_RANGE_SECONDS = STORE_CONSTANTS.MAX_TIME_RANGE_SECONDS;
export const MIN_TIME_RANGE_SECONDS = STORE_CONSTANTS.MIN_TIME_RANGE_SECONDS;
export { VALID_TIMEFRAMES, VALID_COLUMNS };

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

  // Validate symbol using centralized validation
  if (!config.symbol) {
    errors.push('Symbol cannot be empty');
  } else if (!validateSymbol(config.symbol)) {
    errors.push(`Invalid symbol format: ${config.symbol}. Must be in format XXX-XXX (e.g., BTC-USD)`);
  }

  // Validate timeframe using centralized validation
  if (!isValidTimeframe(config.timeframe)) {
    errors.push(`Invalid timeframe '${config.timeframe}'. Must be one of: ${VALID_TIMEFRAMES.join(', ')}`);
  }

  // Validate time range using centralized validation
  if (!validateTimeRange(config.startTime, config.endTime)) {
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
  }

  // Validate indicators count and content
  if (config.indicators.length > STORE_CONSTANTS.MAX_INDICATORS) {
    errors.push(`Too many indicators: ${config.indicators.length} (maximum: ${STORE_CONSTANTS.MAX_INDICATORS})`);
  }
  
  config.indicators.forEach(indicator => {
    if (!indicator || indicator.trim().length === 0) {
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
    columns: ['time', ...state.chartConfig.selectedMetrics] // Include selected metrics
  };
}