// Market data types
export interface MarketData {
  symbol: string;
  price: number;
  change: number;
  changePercent: number;
  volume: number;
  timestamp: number;
}

// Chart render type enum matching Rust RenderType
export enum ChartRenderType {
  Line = 'Line',
  Bar = 'Bar',
  Candlestick = 'Candlestick',
  Triangle = 'Triangle',
  Area = 'Area'
}

// Simplified StoreState matching Rust expectations
export interface SimpleStoreState {
  preset: ChartPreset | null;
  currentSymbol: string;
  startTime: number;
  endTime: number;
}

// Compute operation for calculated fields
export interface ComputeOp {
  type: 'Average' | 'Sum' | 'Difference' | 'Product' | 'Ratio' | 'Min' | 'Max' | 'WeightedAverage';
  weights?: number[]; // For WeightedAverage
}

// Style configuration for rendering
export interface RenderStyle {
  color?: [number, number, number, number]; // RGBA
  colorOptions?: Array<[number, number, number, number]>; // Multiple colors for trades
  size: number; // Line width, triangle size, bar width, etc.
}

// Individual metric configuration
export interface VisibleMetric {
  renderType: ChartRenderType;
  dataColumns: Array<[string, string]>; // [data_type, column_name]
  additionalDataColumns?: Array<[string, string]>; // Additional columns not used for Y bounds
  visible: boolean;
  label: string;
  style: RenderStyle;
  computeOp?: ComputeOp; // For calculated fields like mid price
}

// Metric preset configuration
export interface MetricPreset {
  name: string;
  description: string;
  chartTypes: VisibleMetric[];
}

// Chart configuration
export interface ChartState {
  symbol: string;
  startTime: number;
  endTime: number;
  metricPreset: string | null; // Just the preset name, WASM manages the rest
}

// Simplified chart state for Rust communication
export interface SimpleChartState {
  preset: string | null; // Just the preset name
  symbol: string;
  startTime: number;
  endTime: number;
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
  onMetricsChange?: (newMetrics: string[], oldMetrics: string[]) => void;
  onConnectionChange?: (connected: boolean) => void;
  onMarketDataChange?: (symbol: string, data: MarketData) => void;
  onAnyChange?: (newState: AppState, oldState: AppState) => void;
}

// Full application state for React
export interface StoreState {
  currentSymbol: string;
  ChartStateConfig: ChartState;
  marketData: Record<string, MarketData>;
  isConnected: boolean;
  user?: any;
  // Subscription management for testing
  _subscriptions?: Map<string, StoreSubscriptionCallbacks>;
  _lastState?: StoreState | null;
}

// Keep AppState as alias for backward compatibility
export type AppState = StoreState;

// Preset types matching Rust ChartPreset structure
export interface ChartPreset {
  name: string;
  description: string;
  chart_types: RenderPreset[];
}

// Individual render preset matching Rust RenderPreset
export interface RenderPreset {
  render_type: ChartRenderType;
  data_columns: Array<[string, string]>; // [data_type, column_name]
  additional_data_columns?: Array<[string, string]>; // Additional columns not used for Y bounds
  visible: boolean;
  label: string;
  color?: [number, number, number, number]; // RGBA
  colorOptions?: Array<[number, number, number, number]>; // Multiple colors for trades
  size: number; // Line width, triangle size, bar width, etc.
  compute_op?: ComputeOp; // For calculated fields like mid price
}

// RenderingPreset is now an alias for ChartPreset for backward compatibility
export type RenderingPreset = ChartPreset;

export interface PresetGroup {
  name: string;
  presets: RenderingPreset[];
}

export interface PresetListResponse {
  presets?: RenderingPreset[]; // For backward compatibility
  groups?: PresetGroup[]; // New grouped structure
}

export interface PresetApplyResponse {
  success: boolean;
  message?: string;
  applied_preset?: string;
}

export interface PresetDataResponse {
  success: boolean;
  data?: any; // Binary data or parsed data
  error?: string;
}

export interface ChartStateInfo {
  label: string;
  visible: boolean;
  render_type: string;
  data_columns?: Array<[string, string]>;
}

export interface PresetChartStatesResponse {
  success: boolean;
  preset_name?: string;
  chart_states?: ChartStateInfo[];
  error?: string;
}

export interface ToggleChartTypeResponse {
  success: boolean;
  chart_label?: string;
  visible?: boolean;
  visible_count?: number;
  all_chart_states?: ChartStateInfo[];
  error?: string;
}

// Import constants from centralized configuration
import {
  STORE_CONSTANTS,
  VALID_COLUMNS,
  validateSymbol,
  validateTimeRange
} from '../config/store-constants';

// Re-export commonly used constants for backward compatibility
export const MAX_TIME_RANGE_SECONDS = STORE_CONSTANTS.MAX_TIME_RANGE_SECONDS;
export const MIN_TIME_RANGE_SECONDS = STORE_CONSTANTS.MIN_TIME_RANGE_SECONDS;
export { VALID_COLUMNS };

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
  const configValidation = validateChartStateConfig(state.ChartStateConfig);
  errors.push(...configValidation.errors);
  warnings.push(...configValidation.warnings);

  // Check consistency
  if (state.currentSymbol !== state.ChartStateConfig.symbol) {
    warnings.push(`Current symbol '${state.currentSymbol}' differs from chart config symbol '${state.ChartStateConfig.symbol}'`);
  }

  return { isValid: errors.length === 0, errors, warnings };
}

export function validateChartStateConfig(config: ChartState): ValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  // Validate symbol using centralized validation
  if (!config.symbol) {
    errors.push('Symbol cannot be empty');
  } else if (!validateSymbol(config.symbol)) {
    errors.push(`Invalid symbol format: ${config.symbol}. Must be in format XXX-XXX (e.g., BTC-USD)`);
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


  // Validate metric preset
  if (!config.metricPreset) {
    warnings.push('No metric preset selected');
  }

  return { isValid: errors.length === 0, errors, warnings };
}

export function serializeStoreState(state: StoreState): string {
  return JSON.stringify(state);
}

export function deserializeStoreState(json: string): StoreState {
  return JSON.parse(json);
}

