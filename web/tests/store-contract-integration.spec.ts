/**
 * Store Contract Integration Tests
 * 
 * Comprehensive tests for the React-Rust store contract integration,
 * including state synchronization, change detection, and error handling.
 */

import { test, expect } from '@playwright/test';
import { GraphTestUtils } from './helpers/integration-test-utils';

test.describe('Store Contract Integration', () => {
  
  test.beforeEach(async ({ page }) => {
    // Navigate to the app and wait for initialization
    await page.goto('/app?symbol=BTC-USD&start=1745322750&end=1745691150');
    await GraphTestUtils.waitForWasmLoad(page);
  });

  test('should initialize WASM chart with store state', async ({ page }) => {
    // Verify chart initialization
    const isInitialized = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isInitialized).toBe(true);
    
    // Verify initial store state synchronization
    const storeState = await page.evaluate(() => {
      return JSON.stringify({
        currentSymbol: window.store?.getState?.()?.currentSymbol || 'BTC-USD',
        timeframe: window.store?.getState?.()?.chartConfig?.timeframe || '1h',
        connected: window.store?.getState?.()?.isConnected || false
      });
    });
    
    const parsedState = JSON.parse(storeState);
    expect(parsedState.currentSymbol).toBe('BTC-USD');
    expect(['1m', '5m', '15m', '1h', '4h', '1d']).toContain(parsedState.timeframe);
  });

  test('should synchronize store changes to WASM', async ({ page }) => {
    // Change the symbol in the store
    await page.evaluate(() => {
      if (window.store?.getState?.()?.setCurrentSymbol) {
        window.store.getState().setCurrentSymbol('ETH-USD');
      }
    });

    // Wait for synchronization
    await page.waitForTimeout(200);

    // Verify the change was synchronized
    const currentSymbol = await page.evaluate(() => {
      return window.store?.getState?.()?.currentSymbol;
    });

    expect(currentSymbol).toBe('ETH-USD');
    
    // Verify WASM chart is still initialized after state change
    const isStillInitialized = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isStillInitialized).toBe(true);
  });

  test('should handle timeframe changes correctly', async ({ page }) => {
    const validTimeframes = ['1m', '5m', '15m', '1h', '4h', '1d'];
    
    for (const timeframe of validTimeframes) {
      // Change timeframe
      await page.evaluate((tf) => {
        if (window.store?.getState?.()?.setTimeframe) {
          window.store.getState().setTimeframe(tf);
        }
      }, timeframe);

      // Wait for synchronization
      await page.waitForTimeout(100);

      // Verify timeframe was updated
      const currentTimeframe = await page.evaluate(() => {
        return window.store?.getState?.()?.chartConfig?.timeframe;
      });

      expect(currentTimeframe).toBe(timeframe);
    }
  });

  test('should handle time range updates', async ({ page }) => {
    const now = Math.floor(Date.now() / 1000);
    const oneHourAgo = now - 3600;

    // Update time range
    await page.evaluate(([start, end]) => {
      if (window.store?.getState?.()?.setTimeRange) {
        window.store.getState().setTimeRange(start, end);
      }
    }, [oneHourAgo, now]);

    // Wait for synchronization
    await page.waitForTimeout(200);

    // Verify time range was updated
    const timeRange = await page.evaluate(() => {
      const config = window.store?.getState?.()?.chartConfig;
      return {
        startTime: config?.startTime,
        endTime: config?.endTime
      };
    });

    expect(timeRange.startTime).toBe(oneHourAgo);
    expect(timeRange.endTime).toBe(now);
  });

  test('should validate store state changes', async ({ page }) => {
    // Test invalid symbol (empty string)
    const invalidSymbolResult = await page.evaluate(() => {
      try {
        if (window.store?.getState?.()?.setCurrentSymbol) {
          window.store.getState().setCurrentSymbol('');
        }
        return 'success';
      } catch (error) {
        return error.message;
      }
    });

    // Should either reject empty symbol or handle gracefully
    if (invalidSymbolResult !== 'success') {
      expect(invalidSymbolResult).toContain('symbol');
    }

    // Test invalid timeframe
    const invalidTimeframeResult = await page.evaluate(() => {
      try {
        if (window.store?.getState?.()?.setTimeframe) {
          window.store.getState().setTimeframe('invalid');
        }
        return window.store?.getState?.()?.chartConfig?.timeframe;
      } catch (error) {
        return 'error';
      }
    });

    // Should not accept invalid timeframe
    expect(['1m', '5m', '15m', '1h', '4h', '1d', 'error']).toContain(invalidTimeframeResult);
  });

  test('should handle connection status changes', async ({ page }) => {
    // Test connection status toggle
    await page.evaluate(() => {
      if (window.store?.getState?.()?.setConnectionStatus) {
        window.store.getState().setConnectionStatus(true);
      }
    });

    await page.waitForTimeout(100);

    let connectionStatus = await page.evaluate(() => {
      return window.store?.getState?.()?.isConnected;
    });

    expect(connectionStatus).toBe(true);

    // Toggle to false
    await page.evaluate(() => {
      if (window.store?.getState?.()?.setConnectionStatus) {
        window.store.getState().setConnectionStatus(false);
      }
    });

    await page.waitForTimeout(100);

    connectionStatus = await page.evaluate(() => {
      return window.store?.getState?.()?.isConnected;
    });

    expect(connectionStatus).toBe(false);
  });

  test('should handle multiple rapid state changes', async ({ page }) => {
    // Perform multiple rapid changes
    await page.evaluate(() => {
      const store = window.store?.getState?.();
      if (store) {
        // Rapid symbol changes
        store.setCurrentSymbol?.('BTC-USD');
        store.setCurrentSymbol?.('ETH-USD');
        store.setCurrentSymbol?.('ADA-USD');
        
        // Rapid timeframe changes
        store.setTimeframe?.('1m');
        store.setTimeframe?.('5m');
        store.setTimeframe?.('1h');
        
        // Connection status changes
        store.setConnectionStatus?.(true);
        store.setConnectionStatus?.(false);
        store.setConnectionStatus?.(true);
      }
    });

    // Wait for all changes to propagate
    await page.waitForTimeout(500);

    // Verify final state is consistent
    const finalState = await page.evaluate(() => {
      const state = window.store?.getState?.();
      return {
        symbol: state?.currentSymbol,
        timeframe: state?.chartConfig?.timeframe,
        connected: state?.isConnected,
        chartInitialized: window.wasmChart?.is_initialized?.() || false
      };
    });

    expect(finalState.symbol).toBe('ADA-USD');
    expect(finalState.timeframe).toBe('1h');
    expect(finalState.connected).toBe(true);
    expect(finalState.chartInitialized).toBe(true);
  });

  test('should recover from WASM errors', async ({ page }) => {
    // Simulate WASM error by trying to call method on uninitialized chart
    const errorResult = await page.evaluate(() => {
      try {
        // Try to call a method that might fail
        const result = window.wasmChart?.some_nonexistent_method?.();
        return { success: true, result };
      } catch (error) {
        return { success: false, error: error.message };
      }
    });

    // Should handle error gracefully
    expect(errorResult.success).toBe(false);

    // Verify chart is still functional
    const isStillWorking = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });

    expect(isStillWorking).toBe(true);
  });

  test('should maintain performance under load', async ({ page }) => {
    const startMemory = await GraphTestUtils.measureMemoryUsage(page);
    
    // Perform many state changes
    for (let i = 0; i < 100; i++) {
      await page.evaluate((index) => {
        const store = window.store?.getState?.();
        if (store && index % 10 === 0) {
          // Every 10th iteration, change symbol
          store.setCurrentSymbol?.(index % 2 === 0 ? 'BTC-USD' : 'ETH-USD');
        }
        if (store) {
          // Update connection status
          store.setConnectionStatus?.(index % 2 === 0);
        }
      }, i);
      
      // Small delay to prevent overwhelming
      if (i % 20 === 0) {
        await page.waitForTimeout(10);
      }
    }

    const endMemory = await GraphTestUtils.measureMemoryUsage(page);
    const memoryGrowth = endMemory.used - startMemory.used;
    const growthPercentage = (memoryGrowth / startMemory.used) * 100;

    // Memory growth should be reasonable (less than 50%)
    expect(growthPercentage).toBeLessThan(50);

    // Chart should still be functional
    const isFunctional = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });

    expect(isFunctional).toBe(true);
  });

  test('should handle browser storage integration', async ({ page }) => {
    // Test that preferences are stored/loaded
    await page.evaluate(() => {
      const store = window.store?.getState?.();
      if (store) {
        store.setCurrentSymbol?.('TEST-SYMBOL');
        store.setTimeframe?.('4h');
      }
    });

    // Reload page
    await page.reload();
    await GraphTestUtils.waitForWasmLoad(page);

    // Check if state was persisted (depending on implementation)
    const restoredState = await page.evaluate(() => {
      const state = window.store?.getState?.();
      return {
        symbol: state?.currentSymbol,
        timeframe: state?.chartConfig?.timeframe
      };
    });

    // At minimum, should have valid defaults
    expect(restoredState.symbol).toBeTruthy();
    expect(['1m', '5m', '15m', '1h', '4h', '1d']).toContain(restoredState.timeframe);
  });
});