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


/**
 * Validation rules for different data types
 */
export const VALIDATION_RULES = {
  timeRange: {
    minDuration: STORE_CONSTANTS.MIN_TIME_RANGE_SECONDS,
    maxDuration: STORE_CONSTANTS.MAX_TIME_RANGE_SECONDS,
    errorMessage: `Time range must be between ${STORE_CONSTANTS.MIN_TIME_RANGE_SECONDS} seconds and ${STORE_CONSTANTS.MAX_TIME_RANGE_SECONDS} seconds`
  },

} as const;


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
 * Validate time range
 */
export function validateTimeRange(startTime: number, endTime: number): boolean {
  const duration = endTime - startTime;
  return duration >= VALIDATION_RULES.timeRange.minDuration &&
    duration <= VALIDATION_RULES.timeRange.maxDuration &&
    startTime < endTime;
}