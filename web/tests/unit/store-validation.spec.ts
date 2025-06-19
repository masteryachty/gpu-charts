/**
 * Store State Validation Unit Tests
 * 
 * Tests for the TypeScript store validation logic, type guards,
 * and serialization/deserialization functionality.
 */

import { test, expect } from '@playwright/test';
import {
  validateStoreState,
  validateChartConfig,
  serializeStoreState,
  deserializeStoreState,
  extractFetchParams,
  VALID_TIMEFRAMES,
  VALID_COLUMNS,
  MAX_TIME_RANGE_SECONDS,
  MIN_TIME_RANGE_SECONDS
} from '../../src/types';
import type { StoreState, ChartConfig } from '../../src/types';

test.describe('Store State Validation', () => {
  let validStoreState: StoreState;
  let validChartConfig: ChartConfig;

  test.beforeEach(() => {
    validChartConfig = {
      symbol: 'BTC-USD',
      timeframe: '1h',
      startTime: 1000000,
      endTime: 1003600, // 1 hour later
      indicators: ['RSI', 'MACD'],
      selectedMetrics: ['best_bid', 'best_ask'] // Add dual-metric support
    };

    validStoreState = {
      currentSymbol: 'BTC-USD',
      chartConfig: validChartConfig,
      marketData: {},
      isConnected: true,
      user: {
        id: 'user123',
        name: 'Test User',
        email: 'test@example.com',
        plan: 'pro' as any
      }
    };
  });

  test.describe('Chart Config Validation', () => {
    test('should validate a correct chart config', () => {
      const result = validateChartConfig(validChartConfig);
      expect(result.isValid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    test('should reject empty symbol', () => {
      const config = { ...validChartConfig, symbol: '' };
      const result = validateChartConfig(config);
      expect(result.isValid).toBe(false);
      expect(result.errors).toContain('Symbol cannot be empty');
    });

    test('should reject invalid timeframe', () => {
      const config = { ...validChartConfig, timeframe: 'invalid' };
      const result = validateChartConfig(config);
      expect(result.isValid).toBe(false);
      expect(result.errors.some(e => e.includes('Invalid timeframe'))).toBe(true);
    });

    test('should validate all supported timeframes', () => {
      for (const timeframe of VALID_TIMEFRAMES) {
        const config = { ...validChartConfig, timeframe };
        const result = validateChartConfig(config);
        expect(result.isValid).toBe(true);
      }
    });

    test('should reject invalid time range', () => {
      const config = { 
        ...validChartConfig, 
        startTime: 2000000, 
        endTime: 1000000 // End before start
      };
      const result = validateChartConfig(config);
      expect(result.isValid).toBe(false);
      expect(result.errors).toContain('Start time must be less than end time');
    });

    test('should reject time range too small', () => {
      const config = {
        ...validChartConfig,
        startTime: 1000000,
        endTime: 1000030 // Only 30 seconds
      };
      const result = validateChartConfig(config);
      expect(result.isValid).toBe(false);
      expect(result.errors.some(e => e.includes('Time range too small'))).toBe(true);
    });

    test('should warn about time range too large', () => {
      const config = {
        ...validChartConfig,
        startTime: 1000000,
        endTime: 1000000 + MAX_TIME_RANGE_SECONDS + 1000 // Exceeds max
      };
      const result = validateChartConfig(config);
      expect(result.isValid).toBe(true); // Valid but with warning
      expect(result.warnings.some(w => w.includes('Time range very large'))).toBe(true);
    });

    test('should warn about empty indicators', () => {
      const config = { ...validChartConfig, indicators: ['RSI', '', 'MACD'] };
      const result = validateChartConfig(config);
      expect(result.isValid).toBe(true);
      expect(result.warnings).toContain('Empty indicator name found');
    });
  });

  test.describe('Store State Validation', () => {
    test('should validate a correct store state', () => {
      const result = validateStoreState(validStoreState);
      expect(result.isValid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    test('should reject empty current symbol', () => {
      const state = { ...validStoreState, currentSymbol: '' };
      const result = validateStoreState(state);
      expect(result.isValid).toBe(false);
      expect(result.errors).toContain('Current symbol cannot be empty');
    });

    test('should warn about symbol mismatch', () => {
      const state = { 
        ...validStoreState, 
        currentSymbol: 'ETH-USD',
        chartConfig: { ...validChartConfig, symbol: 'BTC-USD' }
      };
      const result = validateStoreState(state);
      expect(result.isValid).toBe(true);
      expect(result.warnings.some(w => w.includes('differs from chart config symbol'))).toBe(true);
    });

    test('should propagate chart config errors', () => {
      const state = {
        ...validStoreState,
        chartConfig: { ...validChartConfig, symbol: '' }
      };
      const result = validateStoreState(state);
      expect(result.isValid).toBe(false);
      expect(result.errors).toContain('Symbol cannot be empty');
    });
  });

  test.describe('Serialization/Deserialization', () => {
    test('should serialize and deserialize store state correctly', () => {
      const serialized = serializeStoreState(validStoreState);
      expect(typeof serialized).toBe('string');
      
      const deserialized = deserializeStoreState(serialized);
      expect(deserialized).toEqual(validStoreState);
    });

    test('should handle market data serialization', () => {
      const stateWithMarketData = {
        ...validStoreState,
        marketData: {
          'BTC-USD': {
            symbol: 'BTC-USD',
            price: 50000,
            change: 1000,
            changePercent: 2.0,
            volume: 1000000,
            timestamp: 1234567890
          }
        }
      };

      const serialized = serializeStoreState(stateWithMarketData);
      const deserialized = deserializeStoreState(serialized);
      
      expect(deserialized.marketData['BTC-USD'].price).toBe(50000);
      expect(deserialized.marketData['BTC-USD'].symbol).toBe('BTC-USD');
    });

    test('should handle user data serialization', () => {
      const serialized = serializeStoreState(validStoreState);
      const deserialized = deserializeStoreState(serialized);
      
      expect(deserialized.user?.id).toBe('user123');
      expect(deserialized.user?.name).toBe('Test User');
      expect(deserialized.user?.plan).toBe('pro');
    });

    test('should handle undefined user serialization', () => {
      const stateWithoutUser = { ...validStoreState, user: undefined };
      const serialized = serializeStoreState(stateWithoutUser);
      const deserialized = deserializeStoreState(serialized);
      
      expect(deserialized.user).toBeUndefined();
    });
  });

  test.describe('Data Fetch Parameter Extraction', () => {
    test('should extract correct fetch parameters', () => {
      const params = extractFetchParams(validStoreState);
      
      expect(params.symbol).toBe('BTC-USD');
      expect(params.startTime).toBe(1000000);
      expect(params.endTime).toBe(1003600);
      expect(params.columns).toContain('time');
      expect(params.columns).toContain('best_bid');
    });

    test('should use chart config symbol over current symbol', () => {
      const state = {
        ...validStoreState,
        currentSymbol: 'ETH-USD',
        chartConfig: { ...validChartConfig, symbol: 'BTC-USD' }
      };
      
      const params = extractFetchParams(state);
      expect(params.symbol).toBe('BTC-USD');
    });

    test('should include all valid columns', () => {
      const params = extractFetchParams(validStoreState);
      
      for (const column of params.columns) {
        expect(VALID_COLUMNS).toContain(column as any);
      }
    });
  });

  test.describe('Edge Cases', () => {
    test('should handle extreme time values', () => {
      const config = {
        ...validChartConfig,
        startTime: 0,
        endTime: Number.MAX_SAFE_INTEGER
      };
      
      const result = validateChartConfig(config);
      // Should warn about large range but not error
      expect(result.warnings.some(w => w.includes('Time range very large'))).toBe(true);
    });

    test('should handle many indicators', () => {
      const manyIndicators = Array.from({ length: 100 }, (_, i) => `Indicator${i}`);
      const config = { ...validChartConfig, indicators: manyIndicators };
      
      const result = validateChartConfig(config);
      expect(result.isValid).toBe(true);
    });

    test('should handle special characters in symbol', () => {
      const config = { ...validChartConfig, symbol: 'BTC/USD' };
      const result = validateChartConfig(config);
      expect(result.isValid).toBe(true);
    });

    test('should handle unicode in user data', () => {
      const state = {
        ...validStoreState,
        user: {
          id: 'user123',
          name: '测试用户 🚀',
          email: 'test@测试.com',
          plan: 'pro' as any
        }
      };
      
      const serialized = serializeStoreState(state);
      const deserialized = deserializeStoreState(serialized);
      
      expect(deserialized.user?.name).toBe('测试用户 🚀');
      expect(deserialized.user?.email).toBe('test@测试.com');
    });
  });

  test.describe('Performance', () => {
    test('should validate large store states efficiently', () => {
      const largeMarketData: Record<string, any> = {};
      for (let i = 0; i < 1000; i++) {
        largeMarketData[`SYMBOL${i}`] = {
          symbol: `SYMBOL${i}`,
          price: Math.random() * 1000,
          change: Math.random() * 100 - 50,
          changePercent: Math.random() * 10 - 5,
          volume: Math.random() * 1000000,
          timestamp: Date.now()
        };
      }

      const largeState = {
        ...validStoreState,
        marketData: largeMarketData
      };

      const startTime = performance.now();
      const result = validateStoreState(largeState);
      const endTime = performance.now();

      expect(result.isValid).toBe(true);
      expect(endTime - startTime).toBeLessThan(100); // Should complete in < 100ms
    });

    test('should serialize large states efficiently', () => {
      const largeIndicators = Array.from({ length: 1000 }, (_, i) => `Indicator${i}`);
      const largeState = {
        ...validStoreState,
        chartConfig: { ...validChartConfig, indicators: largeIndicators }
      };

      const startTime = performance.now();
      const serialized = serializeStoreState(largeState);
      const deserialized = deserializeStoreState(serialized);
      const endTime = performance.now();

      expect(deserialized.chartConfig.indicators).toHaveLength(1000);
      expect(endTime - startTime).toBeLessThan(50); // Should complete in < 50ms
    });
  });

  test.describe('Dual-Metric Validation', () => {
    test('should validate selectedMetrics array', () => {
      const config = { ...validChartConfig, selectedMetrics: ['best_bid', 'best_ask'] };
      const result = validateChartConfig(config);
      
      expect(result.isValid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    test('should reject empty selectedMetrics array', () => {
      const config = { ...validChartConfig, selectedMetrics: [] };
      const result = validateChartConfig(config);
      
      expect(result.isValid).toBe(false);
      expect(result.errors.some(e => e.includes('selectedMetrics'))).toBe(true);
    });

    test('should reject invalid metric names', () => {
      const config = { ...validChartConfig, selectedMetrics: ['invalid_metric', 'best_bid'] };
      const result = validateChartConfig(config);
      
      expect(result.isValid).toBe(false);
      expect(result.errors.some(e => e.includes('invalid_metric'))).toBe(true);
    });

    test('should accept all valid metric combinations', () => {
      const validCombinations = [
        ['best_bid'],
        ['best_ask'],
        ['price'],
        ['volume'],
        ['best_bid', 'best_ask'],
        ['best_bid', 'price', 'volume'],
        ['best_bid', 'best_ask', 'price', 'volume']
      ];

      for (const metrics of validCombinations) {
        const config = { ...validChartConfig, selectedMetrics: metrics };
        const result = validateChartConfig(config);
        
        expect(result.isValid).toBe(true);
      }
    });

    test('should include selectedMetrics in fetch parameters', () => {
      const state = {
        ...validStoreState,
        chartConfig: { ...validChartConfig, selectedMetrics: ['best_bid', 'price'] }
      };
      
      const params = extractFetchParams(state);
      
      expect(params.columns).toContain('time'); // Always included
      expect(params.columns).toContain('best_bid');
      expect(params.columns).toContain('price');
      expect(params.columns).not.toContain('best_ask'); // Not selected
      expect(params.columns).not.toContain('volume'); // Not selected
    });

    test('should handle metric serialization correctly', () => {
      const state = {
        ...validStoreState,
        chartConfig: { ...validChartConfig, selectedMetrics: ['best_bid', 'best_ask', 'price'] }
      };
      
      const serialized = serializeStoreState(state);
      const deserialized = deserializeStoreState(serialized);
      
      expect(deserialized.chartConfig.selectedMetrics).toEqual(['best_bid', 'best_ask', 'price']);
    });

    test('should maintain camelCase for selectedMetrics field', () => {
      const state = {
        ...validStoreState,
        chartConfig: { ...validChartConfig, selectedMetrics: ['best_bid', 'best_ask'] }
      };
      
      const serialized = serializeStoreState(state);
      const parsed = JSON.parse(serialized);
      
      expect(parsed.chartConfig).toHaveProperty('selectedMetrics');
      expect(parsed.chartConfig.selectedMetrics).toEqual(['best_bid', 'best_ask']);
    });

    test('should validate metric array length constraints', () => {
      // Test maximum reasonable metrics
      const manyMetrics = Array.from({ length: 50 }, (_, i) => `metric_${i}`);
      const config = { ...validChartConfig, selectedMetrics: manyMetrics };
      const result = validateChartConfig(config);
      
      // Should warn about performance impact but not necessarily error
      expect(result.warnings.some(w => w.includes('many metrics'))).toBe(true);
    });

    test('should handle metric validation edge cases', () => {
      const edgeCases = [
        { metrics: null, shouldBeValid: false },
        { metrics: undefined, shouldBeValid: false },
        { metrics: [''], shouldBeValid: false },
        { metrics: ['  '], shouldBeValid: false },
        { metrics: ['best_bid', 'best_bid'], shouldBeValid: true }, // Duplicates should be handled
      ];

      for (const { metrics, shouldBeValid } of edgeCases) {
        const config = { ...validChartConfig, selectedMetrics: metrics as any };
        const result = validateChartConfig(config);
        
        expect(result.isValid).toBe(shouldBeValid);
      }
    });

    test('should generate appropriate column lists for API requests', () => {
      const testCases = [
        {
          selectedMetrics: ['best_bid'],
          expectedColumns: ['time', 'best_bid']
        },
        {
          selectedMetrics: ['best_bid', 'best_ask'],
          expectedColumns: ['time', 'best_bid', 'best_ask']
        },
        {
          selectedMetrics: ['best_bid', 'best_ask', 'price', 'volume'],
          expectedColumns: ['time', 'best_bid', 'best_ask', 'price', 'volume']
        }
      ];

      for (const { selectedMetrics, expectedColumns } of testCases) {
        const state = {
          ...validStoreState,
          chartConfig: { ...validChartConfig, selectedMetrics }
        };
        
        const params = extractFetchParams(state);
        expect(params.columns.sort()).toEqual(expectedColumns.sort());
      }
    });
  });
});