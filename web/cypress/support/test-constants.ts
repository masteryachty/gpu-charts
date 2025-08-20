/**
 * Fixed time ranges for visual regression tests
 * These ensure consistent data across test runs
 */

// Fixed time range: August 1-2, 2025
// Using a fixed historical date ensures consistent data across all test runs
export const VISUAL_TEST_TIME_RANGE = {
  start: 1755518000, // Aug 1, 2025 00:00:00 UTC
  end: 1755605000,   // Aug 2, 2025 00:00:00 UTC (24 hours)
};

// Alternative time ranges for different test scenarios
export const TEST_TIME_RANGES = {
  oneHour: {
    start: 1755518000,
    end: 1755520000,
  },
  fourHours: {
    start: 1755518000,
    end: 1755540800,
  },
  oneDay: {
    start: 1755518000,
    end: 1755605000,
  },
  oneWeek: {
    start: 1755518000, // Jul 26, 2025
    end: 1755905000,   // Aug 2, 2025
  },
};

// Helper to create URL with fixed time range
export function createTestUrl(symbol: string = 'BTC-USD', timeRange = VISUAL_TEST_TIME_RANGE) {
  return `/app?topic=${symbol}&start=${timeRange.start}&end=${timeRange.end}`;
}