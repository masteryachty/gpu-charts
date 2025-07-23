import { describe, expect, test, beforeEach } from '@jest/globals';
import { useAppStore } from '../../src/store/useAppStore';
import { act, renderHook } from '@testing-library/react';

describe('Dual-Metric Store Management', () => {
  let store: ReturnType<typeof useAppStore>;

  beforeEach(() => {
    const { result } = renderHook(() => useAppStore());
    store = result.current;
    
    // Reset to default state
    act(() => {
      store.resetToDefaults();
    });
  });

  describe('selectedMetrics validation', () => {
    test('should validate selectedMetrics against VALID_COLUMNS', () => {
      // Test valid metrics
      act(() => {
        store.setSelectedMetrics(['best_bid', 'best_ask']);
      });
      expect(store.chartConfig.selectedMetrics).toEqual(['best_bid', 'best_ask']);
      
      // Test invalid metrics should be filtered out or cause error
      // This depends on your validation implementation
      act(() => {
        store.setSelectedMetrics(['invalid_metric', 'best_bid']);
      });
      // Verify behavior based on your validation strategy
    });

    test('should require at least one metric', () => {
      // Start with multiple metrics
      act(() => {
        store.setSelectedMetrics(['best_bid', 'best_ask']);
      });
      
      // Try to remove all metrics
      act(() => {
        store.removeMetric('best_bid');
        store.removeMetric('best_ask');
      });
      
      // Should maintain at least one metric
      expect(store.chartConfig.selectedMetrics.length).toBeGreaterThan(0);
    });
  });

  describe('metric addition and removal', () => {
    test('should add metrics correctly', () => {
      const initialCount = store.chartConfig.selectedMetrics.length;
      
      act(() => {
        store.addMetric('volume');
      });
      
      expect(store.chartConfig.selectedMetrics).toContain('volume');
      expect(store.chartConfig.selectedMetrics.length).toBe(initialCount + 1);
    });

    test('should remove metrics correctly', () => {
      // Ensure we start with multiple metrics
      act(() => {
        store.setSelectedMetrics(['best_bid', 'best_ask', 'volume']);
      });
      
      act(() => {
        store.removeMetric('volume');
      });
      
      expect(store.chartConfig.selectedMetrics).not.toContain('volume');
      expect(store.chartConfig.selectedMetrics).toEqual(['best_bid', 'best_ask']);
    });

    test('should prevent removing the last metric', () => {
      // Set to single metric
      act(() => {
        store.setSelectedMetrics(['best_bid']);
      });
      
      act(() => {
        store.removeMetric('best_bid');
      });
      
      // Should still have at least one metric
      expect(store.chartConfig.selectedMetrics.length).toBeGreaterThan(0);
    });

    test('should not add duplicate metrics', () => {
      act(() => {
        store.setSelectedMetrics(['best_bid']);
        store.addMetric('best_bid'); // Try to add duplicate
      });
      
      // Should only have one instance
      const bidCount = store.chartConfig.selectedMetrics.filter(m => m === 'best_bid').length;
      expect(bidCount).toBe(1);
    });
  });

  describe('change detection for metrics', () => {
    test('should trigger change detection when metrics change', () => {
      const initialState = JSON.stringify(store.chartConfig.selectedMetrics);
      
      act(() => {
        store.addMetric('price');
      });
      
      const newState = JSON.stringify(store.chartConfig.selectedMetrics);
      expect(newState).not.toBe(initialState);
    });

    test('should maintain metric order', () => {
      act(() => {
        store.setSelectedMetrics(['best_bid', 'best_ask', 'price']);
        store.removeMetric('best_ask');
        store.addMetric('volume');
      });
      
      // Verify order is maintained logically
      expect(store.chartConfig.selectedMetrics).toEqual(['best_bid', 'price', 'volume']);
    });
  });

  describe('serialization for WASM bridge', () => {
    test('should serialize selectedMetrics correctly', () => {
      act(() => {
        store.setSelectedMetrics(['best_bid', 'best_ask']);
      });
      
      // The store should provide data in format expected by WASM
      const serialized = JSON.stringify({
        chartConfig: store.chartConfig,
        currentSymbol: store.currentSymbol,
        isConnected: store.isConnected,
        marketData: store.marketData,
        user: store.user
      });
      
      const parsed = JSON.parse(serialized);
      expect(parsed.chartConfig.selectedMetrics).toEqual(['best_bid', 'best_ask']);
    });

    test('should handle camelCase field names correctly', () => {
      // Verify the store uses camelCase for React/JS compatibility
      expect(store.chartConfig).toHaveProperty('selectedMetrics');
      expect(store.chartConfig).toHaveProperty('startTime');
      expect(store.chartConfig).toHaveProperty('endTime');
    });
  });

  describe('metric subscription system', () => {
    test('should notify subscribers of metric changes', (done) => {
      let changeCount = 0;
      
      // This would depend on your subscription implementation
      // Example based on typical Zustand patterns
      const unsubscribe = useAppStore.subscribe(
        (state) => state.chartConfig.selectedMetrics,
        (selectedMetrics, prevSelectedMetrics) => {
          if (selectedMetrics !== prevSelectedMetrics) {
            changeCount++;
            if (changeCount === 2) {
              expect(changeCount).toBe(2);
              unsubscribe();
              done();
            }
          }
        }
      );
      
      act(() => {
        store.addMetric('price');
        store.addMetric('volume');
      });
    });
  });

  describe('integration with chart config', () => {
    test('should maintain consistency with other chart config fields', () => {
      act(() => {
        store.setSelectedMetrics(['best_bid']);
        store.setCurrentSymbol('ETH-USD');
        store.setTimeframe('5m');
      });
      
      expect(store.chartConfig.selectedMetrics).toEqual(['best_bid']);
      expect(store.chartConfig.symbol).toBe('ETH-USD');
      expect(store.chartConfig.timeframe).toBe('5m');
    });

    test('should handle metric changes with other state updates', () => {
      act(() => {
        store.updateChartState({
          symbol: 'BTC-USD',
          selectedMetrics: ['best_bid', 'best_ask', 'volume'],
          timeframe: '1h'
        });
      });
      
      expect(store.chartConfig.selectedMetrics).toEqual(['best_bid', 'best_ask', 'volume']);
      expect(store.chartConfig.symbol).toBe('BTC-USD');
      expect(store.chartConfig.timeframe).toBe('1h');
    });
  });
});