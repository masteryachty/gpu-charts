/**
 * Advanced TypeScript Integration
 * 
 * Sophisticated type system enhancements for the React-Rust integration,
 * providing compile-time safety, advanced inference, and runtime validation.
 */

// Utility types for enhanced type safety
export type DeepReadonly<T> = {
  readonly [P in keyof T]: T[P] extends object ? DeepReadonly<T[P]> : T[P];
};

export type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};

export type NonEmptyArray<T> = [T, ...T[]];

export type RequiredKeys<T, K extends keyof T> = T & Required<Pick<T, K>>;

export type OptionalKeys<T, K extends keyof T> = Omit<T, K> & Partial<Pick<T, K>>;

// Brand types for enhanced type safety
export type Brand<T, B> = T & { __brand: B };

export type SymbolId = Brand<string, 'SymbolId'>;
export type Timestamp = Brand<number, 'Timestamp'>;
export type Price = Brand<number, 'Price'>;
export type Volume = Brand<number, 'Volume'>;
export type Percentage = Brand<number, 'Percentage'>;

// Advanced validation types
export interface TypeValidator<T> {
  validate: (value: unknown) => value is T;
  transform?: (value: T) => T;
  errorMessage: string;
}

export type TypeSchema<T> = {
  [K in keyof T]: TypeValidator<T[K]>;
};

// Runtime type validation system
export class TypeValidationError extends Error {
  constructor(
    public field: string,
    public expectedType: string,
    public receivedValue: unknown,
    public path: string[] = []
  ) {
    super(`Type validation failed for ${path.join('.')}.${field}: expected ${expectedType}, received ${typeof receivedValue}`);
    this.name = 'TypeValidationError';
  }
}

export function createValidator<T>(schema: TypeSchema<T>): TypeValidator<T> {
  return {
    validate: (value: unknown): value is T => {
      if (typeof value !== 'object' || value === null) {
        return false;
      }
      
      const obj = value as Record<string, unknown>;
      
      for (const [key, validator] of Object.entries(schema)) {
        if (!validator.validate(obj[key])) {
          return false;
        }
      }
      
      return true;
    },
    errorMessage: `Object must match schema with keys: ${Object.keys(schema).join(', ')}`
  };
}

// Advanced type guards
export function isSymbolId(value: unknown): value is SymbolId {
  return typeof value === 'string' && /^[A-Z]+-[A-Z]+$/.test(value);
}

export function isTimestamp(value: unknown): value is Timestamp {
  return typeof value === 'number' && Number.isInteger(value) && value > 0;
}

export function isPrice(value: unknown): value is Price {
  return typeof value === 'number' && value >= 0 && Number.isFinite(value);
}

export function isVolume(value: unknown): value is Volume {
  return typeof value === 'number' && value >= 0 && Number.isFinite(value);
}

export function isPercentage(value: unknown): value is Percentage {
  return typeof value === 'number' && Number.isFinite(value);
}

// Type-safe enum implementations
export const TimeframeValues = ['1m', '5m', '15m', '1h', '4h', '1d'] as const;
export type Timeframe = typeof TimeframeValues[number];

export const ColumnValues = ['time', 'best_bid', 'best_ask', 'price', 'volume', 'side'] as const;
export type Column = typeof ColumnValues[number];

export const ErrorSeverityValues = ['low', 'medium', 'high', 'critical'] as const;
export type ErrorSeverity = typeof ErrorSeverityValues[number];

export const ErrorCategoryValues = ['wasm', 'data', 'store', 'network', 'performance', 'validation'] as const;
export type ErrorCategory = typeof ErrorCategoryValues[number];

// Type-safe configuration with defaults
export interface TypedConfiguration {
  chart: {
    defaultTimeframe: Timeframe;
    maxDataPoints: number;
    refreshInterval: number;
  };
  performance: {
    fpsThreshold: number;
    memoryThreshold: number;
    enableOptimizations: boolean;
  };
  data: {
    cacheSize: number;
    retryAttempts: number;
    timeoutMs: number;
  };
  errors: {
    enableReporting: boolean;
    maxErrorHistory: number;
    autoRecovery: boolean;
  };
}

export const defaultConfiguration: DeepReadonly<TypedConfiguration> = {
  chart: {
    defaultTimeframe: '1h',
    maxDataPoints: 10000,
    refreshInterval: 1000
  },
  performance: {
    fpsThreshold: 30,
    memoryThreshold: 500 * 1024 * 1024, // 500MB
    enableOptimizations: true
  },
  data: {
    cacheSize: 100,
    retryAttempts: 3,
    timeoutMs: 10000
  },
  errors: {
    enableReporting: true,
    maxErrorHistory: 1000,
    autoRecovery: true
  }
} as const;

// Advanced type-safe event system
export interface TypedEventMap {
  'store:updated': { newState: any; previousState: any; changes: string[] };
  'wasm:initialized': { chart: any; canvasId: string };
  'wasm:error': { error: Error; method: string; recoverable: boolean };
  'data:fetched': { symbol: SymbolId; data: ArrayBuffer; fromCache: boolean };
  'data:error': { symbol: SymbolId; error: Error; retryable: boolean };
  'performance:warning': { metric: string; threshold: number; actual: number };
  'performance:critical': { metric: string; threshold: number; actual: number };
  'error:reported': { error: any; category: ErrorCategory; severity: ErrorSeverity };
}

export type TypedEventListener<K extends keyof TypedEventMap> = (event: TypedEventMap[K]) => void;

export interface TypedEventEmitter {
  on<K extends keyof TypedEventMap>(event: K, listener: TypedEventListener<K>): void;
  off<K extends keyof TypedEventMap>(event: K, listener: TypedEventListener<K>): void;
  emit<K extends keyof TypedEventMap>(event: K, data: TypedEventMap[K]): void;
}

// Type-safe state management with discriminated unions
export type StoreAction = 
  | { type: 'SET_SYMBOL'; payload: { symbol: SymbolId } }
  | { type: 'SET_TIMEFRAME'; payload: { timeframe: Timeframe } }
  | { type: 'SET_TIME_RANGE'; payload: { startTime: Timestamp; endTime: Timestamp } }
  | { type: 'UPDATE_MARKET_DATA'; payload: { symbol: SymbolId; data: any } }
  | { type: 'SET_CONNECTION_STATUS'; payload: { isConnected: boolean } }
  | { type: 'RESET_STATE'; payload?: undefined };

export interface StoreActionCreators {
  setSymbol: (symbol: SymbolId) => StoreAction;
  setTimeframe: (timeframe: Timeframe) => StoreAction;
  setTimeRange: (startTime: Timestamp, endTime: Timestamp) => StoreAction;
  updateMarketData: (symbol: SymbolId, data: any) => StoreAction;
  setConnectionStatus: (isConnected: boolean) => StoreAction;
  resetState: () => StoreAction;
}

// Type-safe reducer with exhaustive checking
export function storeReducer(state: any, action: StoreAction): any {
  switch (action.type) {
    case 'SET_SYMBOL':
      return { ...state, currentSymbol: action.payload.symbol };
    case 'SET_TIMEFRAME':
      return { 
        ...state, 
        chartConfig: { ...state.chartConfig, timeframe: action.payload.timeframe }
      };
    case 'SET_TIME_RANGE':
      return {
        ...state,
        chartConfig: {
          ...state.chartConfig,
          startTime: action.payload.startTime,
          endTime: action.payload.endTime
        }
      };
    case 'UPDATE_MARKET_DATA':
      return {
        ...state,
        marketData: {
          ...state.marketData,
          [action.payload.symbol]: action.payload.data
        }
      };
    case 'SET_CONNECTION_STATUS':
      return { ...state, isConnected: action.payload.isConnected };
    case 'RESET_STATE':
      return createInitialState();
    default:
      // TypeScript ensures this is never reached if all cases are handled
      const _exhaustiveCheck: never = action;
      return state;
  }
}

// Helper function for initial state
function createInitialState(): any {
  return {
    currentSymbol: 'BTC-USD' as SymbolId,
    chartConfig: {
      symbol: 'BTC-USD' as SymbolId,
      timeframe: '1h' as Timeframe,
      startTime: Date.now() - 3600000 as Timestamp,
      endTime: Date.now() as Timestamp,
      indicators: []
    },
    marketData: {},
    isConnected: false
  };
}

// Type-safe action creators
export const actionCreators: StoreActionCreators = {
  setSymbol: (symbol: SymbolId) => ({ type: 'SET_SYMBOL', payload: { symbol } }),
  setTimeframe: (timeframe: Timeframe) => ({ type: 'SET_TIMEFRAME', payload: { timeframe } }),
  setTimeRange: (startTime: Timestamp, endTime: Timestamp) => ({ 
    type: 'SET_TIME_RANGE', 
    payload: { startTime, endTime } 
  }),
  updateMarketData: (symbol: SymbolId, data: any) => ({ 
    type: 'UPDATE_MARKET_DATA', 
    payload: { symbol, data } 
  }),
  setConnectionStatus: (isConnected: boolean) => ({ 
    type: 'SET_CONNECTION_STATUS', 
    payload: { isConnected } 
  }),
  resetState: () => ({ type: 'RESET_STATE' })
};

// Advanced type-safe API client
export interface TypedApiEndpoint<TRequest, TResponse> {
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  path: string;
  requestValidator: TypeValidator<TRequest>;
  responseValidator: TypeValidator<TResponse>;
}

export interface DataApiRequest {
  symbol: SymbolId;
  startTime: Timestamp;
  endTime: Timestamp;
  columns: Column[];
}

export interface DataApiResponse {
  success: boolean;
  data?: ArrayBuffer;
  metadata?: {
    symbol: SymbolId;
    recordCount: number;
    timeRange: [Timestamp, Timestamp];
  };
  error?: string;
}

// Type-safe API endpoint definitions
export const apiEndpoints = {
  getData: {
    method: 'GET',
    path: '/api/data',
    requestValidator: createValidator<DataApiRequest>({
      symbol: { validate: isSymbolId, errorMessage: 'Invalid symbol format' },
      startTime: { validate: isTimestamp, errorMessage: 'Invalid start time' },
      endTime: { validate: isTimestamp, errorMessage: 'Invalid end time' },
      columns: { 
        validate: (value): value is Column[] => 
          Array.isArray(value) && value.every(col => ColumnValues.includes(col as Column)),
        errorMessage: 'Invalid columns array'
      }
    }),
    responseValidator: createValidator<DataApiResponse>({
      success: { validate: (v): v is boolean => typeof v === 'boolean', errorMessage: 'Invalid success flag' },
      data: { validate: (v): v is ArrayBuffer => v instanceof ArrayBuffer, errorMessage: 'Invalid data buffer' },
      metadata: { validate: (v): v is any => typeof v === 'object', errorMessage: 'Invalid metadata' },
      error: { validate: (v): v is string => typeof v === 'string', errorMessage: 'Invalid error message' }
    })
  } as TypedApiEndpoint<DataApiRequest, DataApiResponse>
} as const;

// Type-safe configuration validation
export function validateConfiguration(config: unknown): config is TypedConfiguration {
  if (typeof config !== 'object' || config === null) {
    return false;
  }
  
  const cfg = config as any;
  
  // Validate chart config
  if (!cfg.chart || typeof cfg.chart !== 'object') return false;
  if (!TimeframeValues.includes(cfg.chart.defaultTimeframe)) return false;
  if (typeof cfg.chart.maxDataPoints !== 'number') return false;
  if (typeof cfg.chart.refreshInterval !== 'number') return false;
  
  // Validate performance config
  if (!cfg.performance || typeof cfg.performance !== 'object') return false;
  if (typeof cfg.performance.fpsThreshold !== 'number') return false;
  if (typeof cfg.performance.memoryThreshold !== 'number') return false;
  if (typeof cfg.performance.enableOptimizations !== 'boolean') return false;
  
  // Validate data config
  if (!cfg.data || typeof cfg.data !== 'object') return false;
  if (typeof cfg.data.cacheSize !== 'number') return false;
  if (typeof cfg.data.retryAttempts !== 'number') return false;
  if (typeof cfg.data.timeoutMs !== 'number') return false;
  
  // Validate errors config
  if (!cfg.errors || typeof cfg.errors !== 'object') return false;
  if (typeof cfg.errors.enableReporting !== 'boolean') return false;
  if (typeof cfg.errors.maxErrorHistory !== 'number') return false;
  if (typeof cfg.errors.autoRecovery !== 'boolean') return false;
  
  return true;
}

// Type-safe deep merge utility
export function deepMerge<T extends Record<string, any>>(
  target: T,
  source: DeepPartial<T>
): T {
  const result = { ...target };
  
  for (const key in source) {
    if (source.hasOwnProperty(key)) {
      const sourceValue = source[key];
      const targetValue = result[key];
      
      if (
        sourceValue &&
        typeof sourceValue === 'object' &&
        !Array.isArray(sourceValue) &&
        targetValue &&
        typeof targetValue === 'object' &&
        !Array.isArray(targetValue)
      ) {
        result[key] = deepMerge(targetValue, sourceValue);
      } else if (sourceValue !== undefined) {
        result[key] = sourceValue as any;
      }
    }
  }
  
  return result;
}

// Type-safe environment variable parsing
export interface EnvironmentConfig {
  NODE_ENV: 'development' | 'production' | 'test';
  API_BASE_URL: string;
  ENABLE_DEBUG: boolean;
  PERFORMANCE_MONITORING: boolean;
}

export function parseEnvironmentConfig(): EnvironmentConfig {
  const env = process.env;
  
  return {
    NODE_ENV: (env.NODE_ENV as any) || 'development',
    API_BASE_URL: env.REACT_APP_API_BASE_URL || 'https://api.rednax.io',
    ENABLE_DEBUG: env.REACT_APP_ENABLE_DEBUG === 'true',
    PERFORMANCE_MONITORING: env.REACT_APP_PERFORMANCE_MONITORING !== 'false'
  };
}

// Advanced type-safe hook return types
export type AsyncState<T, E = Error> = 
  | { status: 'idle'; data: null; error: null }
  | { status: 'loading'; data: null; error: null }
  | { status: 'success'; data: T; error: null }
  | { status: 'error'; data: null; error: E };

export interface AsyncOperationResult<T, E = Error> {
  state: AsyncState<T, E>;
  execute: () => Promise<void>;
  reset: () => void;
}

// Type-safe local storage utilities
export interface TypedStorageSchema {
  'user-preferences': {
    theme: 'dark' | 'light';
    defaultSymbol: SymbolId;
    defaultTimeframe: Timeframe;
  };
  'error-history': Array<{
    timestamp: Timestamp;
    category: ErrorCategory;
    severity: ErrorSeverity;
    message: string;
  }>;
  'performance-baselines': {
    [key: string]: {
      fps: number;
      memoryUsage: number;
      renderLatency: number;
    };
  };
}

export function getTypedStorageItem<K extends keyof TypedStorageSchema>(
  key: K
): TypedStorageSchema[K] | null {
  try {
    const item = localStorage.getItem(key);
    return item ? JSON.parse(item) : null;
  } catch {
    return null;
  }
}

export function setTypedStorageItem<K extends keyof TypedStorageSchema>(
  key: K,
  value: TypedStorageSchema[K]
): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch (error) {
    console.warn(`Failed to store ${key}:`, error);
  }
}

// Export all validators for runtime use
export const validators = {
  isSymbolId,
  isTimestamp,
  isPrice,
  isVolume,
  isPercentage,
  validateConfiguration
};

// Export type utilities
export const typeUtils = {
  createValidator,
  deepMerge,
  parseEnvironmentConfig,
  getTypedStorageItem,
  setTypedStorageItem
};