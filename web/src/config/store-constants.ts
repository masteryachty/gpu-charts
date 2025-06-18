/**
 * Store Configuration Constants
 * 
 * Centralized configuration for store behavior, validation rules,
 * and performance thresholds to avoid magic numbers throughout the codebase.
 */

export const STORE_CONSTANTS = {
  // Validation thresholds
  MAX_TIME_RANGE_SECONDS: 86400 * 30, // 30 days
  MIN_TIME_RANGE_SECONDS: 60, // 1 minute
  MAX_SYMBOL_LENGTH: 20,
  MIN_SYMBOL_LENGTH: 3,
  MAX_INDICATORS: 200,
  
  // Performance thresholds
  MAX_MARKET_DATA_ENTRIES: 1000,
  MAX_ERROR_HISTORY_SIZE: 100,
  DEBOUNCE_DEFAULT_MS: 100,
  VALIDATION_TIMEOUT_MS: 5000,
  
  // Store subscription settings
  MAX_SUBSCRIPTIONS: 50,
  SUBSCRIPTION_CLEANUP_INTERVAL_MS: 60000, // 1 minute
  
  // Default values
  DEFAULT_SYMBOL: 'BTC-USD',
  DEFAULT_TIMEFRAME: '1h',
  DEFAULT_TIME_RANGE_HOURS: 24,
  
  // State change detection
  SIGNIFICANT_TIME_CHANGE_THRESHOLD_SECONDS: 60,
  MARKET_DATA_UPDATE_THRESHOLD_MS: 1000,
  
  // Memory management
  MAX_STORE_SIZE_MB: 50,
  MEMORY_CHECK_INTERVAL_MS: 30000,
  
  // Error handling
  MAX_CONSECUTIVE_ERRORS: 10,
  ERROR_RESET_TIMEOUT_MS: 300000, // 5 minutes
  
} as const;

export const VALID_TIMEFRAMES = ['1m', '5m', '15m', '1h', '4h', '1d'] as const;
export const VALID_COLUMNS = ['time', 'best_bid', 'best_ask', 'price', 'volume', 'side'] as const;
export const VALID_USER_PLANS = ['free', 'pro', 'enterprise'] as const;

export type ValidTimeframe = typeof VALID_TIMEFRAMES[number];
export type ValidColumn = typeof VALID_COLUMNS[number];
export type ValidUserPlan = typeof VALID_USER_PLANS[number];

/**
 * Validation rules for different data types
 */
export const VALIDATION_RULES = {
  symbol: {
    pattern: /^[A-Z]+[-\/][A-Z]+$/,
    minLength: STORE_CONSTANTS.MIN_SYMBOL_LENGTH,
    maxLength: STORE_CONSTANTS.MAX_SYMBOL_LENGTH,
    errorMessage: 'Symbol must be in format XXX-XXX or XXX/XXX (e.g., BTC-USD or BTC/USD)'
  },
  
  timeframe: {
    validValues: VALID_TIMEFRAMES,
    errorMessage: `Timeframe must be one of: ${VALID_TIMEFRAMES.join(', ')}`
  },
  
  timeRange: {
    minDuration: STORE_CONSTANTS.MIN_TIME_RANGE_SECONDS,
    maxDuration: STORE_CONSTANTS.MAX_TIME_RANGE_SECONDS,
    errorMessage: `Time range must be between ${STORE_CONSTANTS.MIN_TIME_RANGE_SECONDS} seconds and ${STORE_CONSTANTS.MAX_TIME_RANGE_SECONDS} seconds`
  },
  
  indicators: {
    maxCount: STORE_CONSTANTS.MAX_INDICATORS,
    errorMessage: `Maximum ${STORE_CONSTANTS.MAX_INDICATORS} indicators allowed`
  }
} as const;

/**
 * Default store configuration
 */
export const DEFAULT_STORE_CONFIG = {
  enableValidation: true,
  enablePerformanceMonitoring: true,
  enableMemoryManagement: true,
  enableAutomaticCleanup: true,
  strictTypeChecking: true,
  
  // Performance settings
  maxRenderUpdatesPerSecond: 60,
  maxStateUpdatesPerSecond: 100,
  enableBatchUpdates: true,
  
  // Development settings
  enableDebugLogging: process.env.NODE_ENV === 'development',
  enableStateHistory: process.env.NODE_ENV === 'development',
  maxStateHistorySize: 100,
  
} as const;

/**
 * Error severity mapping for different validation failures
 */
export const ERROR_SEVERITY_MAP = {
  'INVALID_SYMBOL': 'medium',
  'INVALID_TIMEFRAME': 'medium', 
  'INVALID_TIME_RANGE': 'high',
  'SYMBOL_MISMATCH': 'low',
  'EMPTY_INDICATOR': 'low',
  'TOO_MANY_INDICATORS': 'medium',
  'MEMORY_LIMIT_EXCEEDED': 'critical',
  'PERFORMANCE_DEGRADED': 'medium',
  'STATE_CORRUPTION': 'critical',
} as const;

/**
 * Performance thresholds for monitoring
 */
export const PERFORMANCE_THRESHOLDS = {
  // FPS thresholds
  MIN_FPS: 30,
  TARGET_FPS: 60,
  
  // Memory thresholds (in bytes)
  MEMORY_WARNING_THRESHOLD: 50 * 1024 * 1024, // 50MB
  MEMORY_CRITICAL_THRESHOLD: 100 * 1024 * 1024, // 100MB
  
  // Latency thresholds (in milliseconds)
  MAX_STATE_UPDATE_LATENCY: 16, // 60fps budget
  MAX_RENDER_LATENCY: 16,
  MAX_VALIDATION_LATENCY: 5,
  
  // Throughput thresholds
  MIN_UPDATES_PER_SECOND: 30,
  MAX_UPDATES_PER_SECOND: 1000,
  
  // Error rate thresholds
  MAX_ERROR_RATE_PERCENT: 5,
  MAX_CONSECUTIVE_ERRORS: 10,
  
} as const;

/**
 * Feature flags for enabling/disabling functionality
 */
export const FEATURE_FLAGS = {
  ENABLE_ADVANCED_VALIDATION: true,
  ENABLE_PERFORMANCE_MONITORING: true,
  ENABLE_MEMORY_TRACKING: true,
  ENABLE_ERROR_REPORTING: true,
  ENABLE_STATE_PERSISTENCE: false, // Disabled for now
  ENABLE_REAL_TIME_SYNC: true,
  ENABLE_BATCH_UPDATES: true,
  ENABLE_DEBUG_PANEL: process.env.NODE_ENV === 'development',
  ENABLE_PROFILING: false, // Only enable for performance testing
} as const;

/**
 * Helper function to check if a value is a valid timeframe
 */
export function isValidTimeframe(value: unknown): value is ValidTimeframe {
  return typeof value === 'string' && VALID_TIMEFRAMES.includes(value as ValidTimeframe);
}

/**
 * Helper function to check if a value is a valid column
 */
export function isValidColumn(value: unknown): value is ValidColumn {
  return typeof value === 'string' && VALID_COLUMNS.includes(value as ValidColumn);
}

/**
 * Helper function to check if a value is a valid user plan
 */
export function isValidUserPlan(value: unknown): value is ValidUserPlan {
  return typeof value === 'string' && VALID_USER_PLANS.includes(value as ValidUserPlan);
}

/**
 * Get current timestamp in Unix seconds
 */
export function getCurrentTimestamp(): number {
  return Math.floor(Date.now() / 1000);
}

/**
 * Create a default time range (last 24 hours)
 */
export function createDefaultTimeRange(): [number, number] {
  const now = getCurrentTimestamp();
  const start = now - (STORE_CONSTANTS.DEFAULT_TIME_RANGE_HOURS * 3600);
  return [start, now];
}

/**
 * Validate symbol format
 */
export function validateSymbol(symbol: string): boolean {
  return VALIDATION_RULES.symbol.pattern.test(symbol) &&
         symbol.length >= VALIDATION_RULES.symbol.minLength &&
         symbol.length <= VALIDATION_RULES.symbol.maxLength;
}

/**
 * Validate time range
 */
export function validateTimeRange(startTime: number, endTime: number): boolean {
  const duration = endTime - startTime;
  return duration >= VALIDATION_RULES.timeRange.minDuration &&
         duration <= VALIDATION_RULES.timeRange.maxDuration &&
         startTime < endTime;
}