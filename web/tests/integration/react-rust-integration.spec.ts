import { test, expect, Page } from '@playwright/test';
import { GraphTestUtils } from '../helpers/test-utils';
import { DataMockHelper } from '../helpers/data-mocks';

/**
 * End-to-End React-Rust Integration Tests
 * 
 * Comprehensive testing of the complete React Store → Rust integration system,
 * including store synchronization, data fetching, error handling, and performance.
 */

test.describe('React-Rust Integration System', () => {
  let page: Page;
  
  test.beforeEach(async ({ page: testPage }) => {
    page = testPage;
    
    // Navigate to trading app
    await page.goto('/app?symbol=BTC-USD&start=1745322750&end=1745691150&debug=true');
    
    // Wait for WASM to load
    await GraphTestUtils.waitForWasmLoad(page);
    
    // Enable debug mode for detailed testing
    await page.locator('input[type="checkbox"]:near(:text("Debug Mode"))').check();
  });

  test.describe('Store Contract Foundation', () => {
    test('should validate store state structure', async () => {
      // Check that store state has required properties
      const storeState = await page.evaluate(() => {
        return (window as any).__APP_STORE_STATE__;
      });
      
      expect(storeState).toHaveProperty('currentSymbol');
      expect(storeState).toHaveProperty('chartConfig');
      expect(storeState).toHaveProperty('marketData');
      expect(storeState).toHaveProperty('isConnected');
    });

    test('should handle invalid store state gracefully', async () => {
      // Inject invalid state and verify error handling
      const errorOccurred = await page.evaluate(async () => {
        try {
          const invalidState = {
            currentSymbol: '', // Invalid empty symbol
            chartConfig: {
              symbol: 'INVALID',
              timeframe: '99h', // Invalid timeframe
              startTime: 999999999999999, // Invalid time
              endTime: -1 // Invalid end time
            }
          };
          
          // This should trigger validation errors
          await (window as any).__UPDATE_STORE_STATE__(invalidState);
          return false;
        } catch (error) {
          return true;
        }
      });
      
      expect(errorOccurred).toBe(true);
      
      // Check that error notification appeared
      await expect(page.locator('.bg-red-900')).toBeVisible({ timeout: 3000 });
    });

    test('should serialize and deserialize state correctly', async () => {
      const originalState = await page.evaluate(() => {
        return (window as any).__APP_STORE_STATE__;
      });
      
      const serializedState = await page.evaluate((state) => {
        return JSON.stringify(state);
      }, originalState);
      
      const deserializedState = await page.evaluate((serialized) => {
        return JSON.parse(serialized);
      }, serializedState);
      
      expect(deserializedState).toEqual(originalState);
    });
  });

  test.describe('WASM Bridge Communication', () => {
    test('should initialize WASM chart successfully', async () => {
      // Verify chart is initialized
      const isInitialized = await page.evaluate(async () => {
        const canvas = document.getElementById('wasm-chart-canvas');
        return canvas && canvas.getAttribute('data-initialized') === 'true';
      });
      
      expect(isInitialized).toBe(true);
    });

    test('should update chart state via WASM bridge', async () => {
      // Change symbol and verify WASM receives update
      await page.selectOption('select[data-testid="symbol-selector"]', 'ETH-USD');
      
      // Wait for state synchronization
      await page.waitForTimeout(500);
      
      // Verify WASM chart received the update
      const wasmState = await page.evaluate(async () => {
        return await (window as any).__GET_WASM_CHART_STATE__();
      });
      
      expect(wasmState.currentSymbol).toBe('ETH-USD');
    });

    test('should handle WASM method failures gracefully', async () => {
      // Simulate WASM method failure
      const errorHandled = await page.evaluate(async () => {
        try {
          // Call non-existent WASM method
          await (window as any).__CALL_INVALID_WASM_METHOD__();
          return false;
        } catch (error) {
          return true;
        }
      });
      
      expect(errorHandled).toBe(true);
      
      // Verify error boundary didn't crash the app
      await expect(page.locator('#wasm-chart-canvas')).toBeVisible();
    });

    test('should measure WASM method performance', async () => {
      const performanceMetrics = await page.evaluate(async () => {
        const start = performance.now();
        
        // Call WASM update method
        await (window as any).__UPDATE_WASM_CHART_STATE__({
          currentSymbol: 'BTC-USD',
          chartConfig: {
            symbol: 'BTC-USD',
            timeframe: '1h',
            startTime: 1745322750,
            endTime: 1745691150,
            indicators: []
          }
        });
        
        const end = performance.now();
        return { latency: end - start };
      });
      
      // WASM method should complete within reasonable time
      expect(performanceMetrics.latency).toBeLessThan(100); // 100ms threshold
    });
  });

  test.describe('Smart State Change Detection', () => {
    test('should detect symbol changes', async () => {
      // Get initial state
      const initialSymbol = await page.locator('[data-testid="current-symbol"]').textContent();
      
      // Change symbol
      await page.selectOption('select[data-testid="symbol-selector"]', 'ETH-USD');
      
      // Wait for change detection
      await page.waitForTimeout(200);
      
      // Verify change was detected
      const changeDetection = await page.evaluate(() => {
        return (window as any).__LAST_CHANGE_DETECTION__;
      });
      
      expect(changeDetection.symbolChanged).toBe(true);
      expect(changeDetection.hasChanges).toBe(true);
    });

    test('should detect timeframe changes', async () => {
      // Change timeframe
      await page.selectOption('select[data-testid="timeframe-selector"]', '5m');
      
      await page.waitForTimeout(200);
      
      const changeDetection = await page.evaluate(() => {
        return (window as any).__LAST_CHANGE_DETECTION__;
      });
      
      expect(changeDetection.timeframeChanged).toBe(true);
    });

    test('should detect time range changes', async () => {
      // Simulate zoom interaction to change time range
      await GraphTestUtils.triggerChartInteraction(page, 'zoom', { x: 400, y: 300 });
      
      await page.waitForTimeout(300);
      
      const changeDetection = await page.evaluate(() => {
        return (window as any).__LAST_CHANGE_DETECTION__;
      });
      
      expect(changeDetection.timeRangeChanged).toBe(true);
    });

    test('should not trigger false positives', async () => {
      // Make no actual changes
      await page.waitForTimeout(1000);
      
      const changeDetection = await page.evaluate(() => {
        return (window as any).__LAST_CHANGE_DETECTION__;
      });
      
      // Should not detect changes when none occurred
      expect(changeDetection.hasChanges).toBe(false);
    });
  });

  test.describe('React Store Subscription', () => {
    test('should sync store changes to WASM automatically', async () => {
      // Change store state through UI
      await page.selectOption('select[data-testid="symbol-selector"]', 'ADA-USD');
      
      // Wait for debounced sync
      await page.waitForTimeout(150);
      
      // Verify WASM chart received the update
      const syncIndicator = page.locator('[title="Synced"]');
      await expect(syncIndicator).toBeVisible({ timeout: 2000 });
    });

    test('should handle rapid state changes with debouncing', async () => {
      // Make rapid changes
      for (let i = 0; i < 5; i++) {
        await page.selectOption('select[data-testid="timeframe-selector"]', i % 2 === 0 ? '1h' : '5m');
        await page.waitForTimeout(50);
      }
      
      // Wait for debouncing to settle
      await page.waitForTimeout(200);
      
      // Should only see final state, not all intermediate states
      const updateCount = await page.evaluate(() => {
        return (window as any).__WASM_UPDATE_COUNT__ || 0;
      });
      
      // Should be significantly fewer than 5 updates due to debouncing
      expect(updateCount).toBeLessThan(3);
    });

    test('should show uncommitted changes indicator', async () => {
      // Make a change
      await page.selectOption('select[data-testid="symbol-selector"]', 'DOT-USD');
      
      // Should immediately show syncing indicator
      const syncingIndicator = page.locator('[title="Syncing..."]');
      await expect(syncingIndicator).toBeVisible({ timeout: 500 });
      
      // Should eventually show synced indicator
      const syncedIndicator = page.locator('[title="Synced"]');
      await expect(syncedIndicator).toBeVisible({ timeout: 2000 });
    });
  });

  test.describe('Autonomous Data Fetching', () => {
    test('should fetch data automatically on symbol change', async () => {
      // Mock successful data response
      await DataMockHelper.mockServerResponse(page, 
        DataMockHelper.generateMarketData('BTC-USD', 1000)
      );
      
      // Change symbol to trigger data fetch
      await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
      
      // Wait for data fetch to complete
      await expect(page.locator('[data-testid="data-fetch-indicator"]')).toBeVisible({ timeout: 3000 });
      
      // Verify data was fetched
      const dataFetchInfo = await page.locator('[data-testid="last-fetch-info"]').textContent();
      expect(dataFetchInfo).toContain('BTC-USD');
    });

    test('should cache data effectively', async () => {
      // First fetch
      await page.selectOption('select[data-testid="symbol-selector"]', 'ETH-USD');
      await page.waitForTimeout(1000);
      
      // Second fetch of same data
      await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
      await page.selectOption('select[data-testid="symbol-selector"]', 'ETH-USD');
      
      await page.waitForTimeout(500);
      
      // Should show cache hit
      const cacheStatus = await page.locator('[data-testid="cache-status"]').textContent();
      expect(cacheStatus).toContain('cached');
    });

    test('should handle data fetch failures gracefully', async () => {
      // Mock server error
      await page.route('**/api/data**', async (route) => {
        await route.fulfill({ status: 500, body: 'Server Error' });
      });
      
      // Trigger data fetch
      await page.selectOption('select[data-testid="symbol-selector"]', 'FAIL-USD');
      
      // Should show error notification
      await expect(page.locator('.bg-red-900')).toBeVisible({ timeout: 3000 });
      
      // App should still be functional
      await expect(page.locator('#wasm-chart-canvas')).toBeVisible();
    });

    test('should display data fetching metrics', async () => {
      // Enable data fetching monitor
      await page.locator('input[type="checkbox"]:near(:text("Show Activity"))').check();
      
      // Trigger multiple fetches
      for (const symbol of ['BTC-USD', 'ETH-USD', 'ADA-USD']) {
        await page.selectOption('select[data-testid="symbol-selector"]', symbol);
        await page.waitForTimeout(300);
      }
      
      // Check metrics are displayed
      await expect(page.locator('[data-testid="total-requests"]')).toContainText(/[1-9]/);
      await expect(page.locator('[data-testid="cache-hit-rate"]')).toBeVisible();
    });
  });

  test.describe('Comprehensive Error Handling', () => {
    test('should recover from WASM initialization failures', async () => {
      // Force WASM failure and recovery
      await page.evaluate(() => {
        (window as any).__FORCE_WASM_FAILURE__ = true;
      });
      
      // Trigger reinitialization
      await page.locator('[data-testid="reset-button"]').click();
      
      // Should show retry attempts
      await expect(page.locator(':text("Retry")')).toBeVisible({ timeout: 2000 });
      
      // Eventually should recover or show fallback
      await expect(page.locator(':text("Chart engine unavailable")')).toBeVisible({ timeout: 10000 });
    });

    test('should display user-friendly error notifications', async () => {
      // Trigger a validation error
      await page.evaluate(() => {
        (window as any).__TRIGGER_VALIDATION_ERROR__({
          symbol: '',
          timeframe: 'invalid'
        });
      });
      
      // Should show user-friendly error
      const errorNotification = page.locator('.bg-red-900');
      await expect(errorNotification).toBeVisible();
      
      const errorText = await errorNotification.textContent();
      expect(errorText).not.toContain('TypeError'); // Should not show technical errors
    });

    test('should handle network errors gracefully', async () => {
      // Simulate network offline
      await page.context().setOffline(true);
      
      // Try to fetch data
      await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
      
      await page.waitForTimeout(2000);
      
      // Should show network error
      await expect(page.locator(':text("connection")')).toBeVisible();
      
      // Restore network
      await page.context().setOffline(false);
      
      // Should recover automatically
      await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 5000 });
    });

    test('should provide error recovery actions', async () => {
      // Trigger critical error
      await page.evaluate(() => {
        (window as any).__TRIGGER_CRITICAL_ERROR__();
      });
      
      // Should show recovery options
      await expect(page.locator(':text("Try Again")')).toBeVisible();
      await expect(page.locator(':text("Reset Application")')).toBeVisible();
      await expect(page.locator(':text("Reload Page")')).toBeVisible();
    });
  });

  test.describe('Performance Optimization', () => {
    test('should monitor performance metrics', async () => {
      // Wait for performance monitoring to collect data
      await page.waitForTimeout(2000);
      
      // Check that performance overlay shows metrics
      const fpsDisplay = page.locator('[data-testid="fps-display"]');
      await expect(fpsDisplay).toBeVisible();
      
      const fpsValue = await fpsDisplay.textContent();
      const fps = parseInt(fpsValue?.replace(/\D/g, '') || '0');
      expect(fps).toBeGreaterThan(0);
    });

    test('should trigger performance optimizations when needed', async () => {
      // Simulate high memory usage
      await page.evaluate(() => {
        // Create memory pressure
        const arrays = [];
        for (let i = 0; i < 1000; i++) {
          arrays.push(new Array(10000).fill(Math.random()));
        }
        (window as any).__MEMORY_PRESSURE__ = arrays;
      });
      
      await page.waitForTimeout(3000);
      
      // Should trigger memory cleanup optimization
      const optimizationMessage = page.locator(':text("Memory cleanup")');
      await expect(optimizationMessage).toBeVisible({ timeout: 5000 });
    });

    test('should maintain stable frame rate during interactions', async () => {
      // Perform intensive chart interactions
      for (let i = 0; i < 20; i++) {
        await GraphTestUtils.triggerChartInteraction(page, 'zoom', { 
          x: 300 + i * 10, 
          y: 200 + i * 5 
        });
        await page.waitForTimeout(50);
      }
      
      await page.waitForTimeout(1000);
      
      // Check final FPS is still reasonable
      const fpsValue = await page.locator('[data-testid="fps-display"]').textContent();
      const fps = parseInt(fpsValue?.replace(/\D/g, '') || '0');
      expect(fps).toBeGreaterThan(15); // Should maintain at least 15 FPS
    });

    test('should provide performance recommendations', async () => {
      // Simulate performance issues
      await page.evaluate(() => {
        (window as any).__SIMULATE_PERFORMANCE_ISSUES__ = {
          lowFps: true,
          highMemory: true,
          highLatency: true
        };
      });
      
      await page.waitForTimeout(2000);
      
      // Should show performance recommendations
      await expect(page.locator(':text("recommendation")')).toBeVisible({ timeout: 3000 });
    });
  });

  test.describe('Complete System Integration', () => {
    test('should handle complex workflow end-to-end', async () => {
      // Complex workflow: symbol change → data fetch → WASM update → performance monitoring
      
      // Step 1: Change symbol
      await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
      
      // Step 2: Verify data fetch triggered
      await expect(page.locator('[data-testid="data-fetch-indicator"]')).toBeVisible({ timeout: 2000 });
      
      // Step 3: Verify WASM chart updated
      await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 3000 });
      
      // Step 4: Verify performance metrics updated
      const performanceData = await page.evaluate(() => {
        return (window as any).__PERFORMANCE_METRICS__;
      });
      
      expect(performanceData).toHaveProperty('dataProcessingTime');
      expect(performanceData).toHaveProperty('renderLatency');
      
      // Step 5: Verify no errors occurred
      const errorCount = await page.locator('.bg-red-900').count();
      expect(errorCount).toBe(0);
    });

    test('should recover from multiple system failures', async () => {
      // Simulate multiple failures
      await page.evaluate(() => {
        (window as any).__SIMULATE_MULTIPLE_FAILURES__ = {
          wasmFailure: true,
          dataFetchFailure: true,
          performanceIssues: true
        };
      });
      
      // Wait for error handling to kick in
      await page.waitForTimeout(3000);
      
      // Should show error notifications but app should still be functional
      await expect(page.locator('.bg-red-900')).toBeVisible();
      await expect(page.locator('#wasm-chart-canvas')).toBeVisible();
      
      // Clear failures and verify recovery
      await page.evaluate(() => {
        (window as any).__SIMULATE_MULTIPLE_FAILURES__ = null;
      });
      
      await page.locator(':text("Try Again")').first().click();
      
      // Should eventually recover
      await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 10000 });
    });

    test('should maintain data consistency across all systems', async () => {
      // Make state changes and verify consistency across all layers
      
      const testSymbol = 'ETH-USD';
      const testTimeframe = '5m';
      
      // Change state through UI
      await page.selectOption('select[data-testid="symbol-selector"]', testSymbol);
      await page.selectOption('select[data-testid="timeframe-selector"]', testTimeframe);
      
      await page.waitForTimeout(500);
      
      // Verify consistency across all systems
      const storeState = await page.evaluate(() => (window as any).__APP_STORE_STATE__);
      const wasmState = await page.evaluate(() => (window as any).__GET_WASM_CHART_STATE__());
      const dataFetchState = await page.evaluate(() => (window as any).__DATA_FETCH_STATE__);
      
      // All systems should have consistent state
      expect(storeState.currentSymbol).toBe(testSymbol);
      expect(storeState.chartConfig.timeframe).toBe(testTimeframe);
      expect(wasmState.currentSymbol).toBe(testSymbol);
      expect(wasmState.chartConfig.timeframe).toBe(testTimeframe);
      expect(dataFetchState.lastFetch?.symbol).toBe(testSymbol);
    });

    test('should scale performance with data size', async () => {
      // Test with different data sizes
      const testCases = [
        { records: 100, expectedTime: 50 },
        { records: 1000, expectedTime: 200 },
        { records: 10000, expectedTime: 1000 }
      ];
      
      for (const testCase of testCases) {
        // Mock data of specific size
        await DataMockHelper.mockServerResponse(page, 
          DataMockHelper.generateMarketData('TEST-USD', testCase.records)
        );
        
        const startTime = Date.now();
        
        // Trigger data processing
        await page.selectOption('select[data-testid="symbol-selector"]', 'TEST-USD');
        await page.waitForTimeout(500);
        
        const endTime = Date.now();
        const processingTime = endTime - startTime;
        
        // Should scale reasonably with data size
        expect(processingTime).toBeLessThan(testCase.expectedTime);
      }
    });
  });
});