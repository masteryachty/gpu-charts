import { test, expect } from '@playwright/test';
import { IntegrationTestUtils, IntegrationDataMockHelper } from '../helpers/integration-test-utils';

/**
 * Performance Benchmark Tests
 * 
 * Comprehensive performance testing for the React-Rust integration system,
 * ensuring the system performs well under various load conditions.
 */

test.describe('Performance Benchmarks', () => {
  
  test.beforeEach(async ({ page }) => {
    // Navigate to trading app with performance monitoring enabled
    await page.goto('/app?symbol=BTC-USD&debug=true&performance=true');
    
    // Inject test hooks for detailed monitoring
    await IntegrationTestUtils.injectTestHooks(page);
    
    // Wait for complete system initialization
    await IntegrationTestUtils.waitForSystemReady(page);
  });

  test.describe('Baseline Performance', () => {
    
    test('should achieve target frame rate in idle state', async ({ page }) => {
      // Wait for performance monitoring to stabilize
      await page.waitForTimeout(3000);
      
      const metrics = await IntegrationTestUtils.getTestMetrics(page);
      
      // Should maintain at least 45 FPS in idle state
      expect(metrics.performanceScore).toBeGreaterThan(75);
      
      // Verify specific FPS metric
      const performanceData = await page.evaluate(() => (window as any).__PERFORMANCE_METRICS__);
      expect(performanceData.fps).toBeGreaterThan(45);
    });

    test('should initialize within performance budget', async ({ page }) => {
      const startTime = Date.now();
      
      // Navigate to a fresh instance
      await page.goto('/app?symbol=ETH-USD&debug=true');
      
      // Wait for complete initialization
      await IntegrationTestUtils.waitForSystemReady(page, 10000);
      
      const initializationTime = Date.now() - startTime;
      
      // Should initialize within 8 seconds
      expect(initializationTime).toBeLessThan(8000);
      
      // Verify memory usage is reasonable
      const performanceData = await page.evaluate(() => (window as any).__PERFORMANCE_METRICS__);
      expect(performanceData.totalMemoryUsage).toBeLessThan(200 * 1024 * 1024); // 200MB
    });

    test('should maintain stable memory usage in idle state', async ({ page }) => {
      // Record initial memory usage
      const initialMemory = await page.evaluate(() => {
        const perf = (window as any).__PERFORMANCE_METRICS__;
        return perf ? perf.totalMemoryUsage : 0;
      });
      
      // Wait for 30 seconds of idle time
      await page.waitForTimeout(30000);
      
      // Check final memory usage
      const finalMemory = await page.evaluate(() => {
        const perf = (window as any).__PERFORMANCE_METRICS__;
        return perf ? perf.totalMemoryUsage : 0;
      });
      
      // Memory should not grow significantly in idle state
      const memoryGrowth = (finalMemory - initialMemory) / initialMemory * 100;
      expect(memoryGrowth).toBeLessThan(10); // Less than 10% growth
    });
  });

  test.describe('Load Testing', () => {
    
    test('should handle rapid symbol changes', async ({ page }) => {
      const symbols = ['BTC-USD', 'ETH-USD', 'ADA-USD', 'DOT-USD', 'LINK-USD'];
      const startTime = Date.now();
      
      // Perform rapid symbol changes
      for (let cycle = 0; cycle < 3; cycle++) {
        for (const symbol of symbols) {
          await page.selectOption('select[data-testid="symbol-selector"]', symbol);
          await page.waitForTimeout(100); // Minimal delay
        }
      }
      
      // Wait for all operations to settle
      await page.waitForTimeout(2000);
      
      const totalTime = Date.now() - startTime;
      const operations = symbols.length * 3;
      const avgTimePerOperation = totalTime / operations;
      
      // Should handle each symbol change quickly
      expect(avgTimePerOperation).toBeLessThan(500); // 500ms per operation
      
      // Verify system is still responsive
      const metrics = await IntegrationTestUtils.getTestMetrics(page);
      expect(metrics.performanceScore).toBeGreaterThan(60);
    });

    test('should handle large dataset processing', async ({ page }) => {
      // Mock large dataset
      await IntegrationDataMockHelper.mockProgressiveDataLoading(page, 'BTC-USD', 50000);
      
      const startTime = Date.now();
      
      // Trigger data processing
      await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
      
      // Wait for processing to complete
      await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 15000 });
      
      const processingTime = Date.now() - startTime;
      
      // Should process 50k records within reasonable time
      expect(processingTime).toBeLessThan(10000); // 10 seconds
      
      // Verify performance after processing
      const performanceData = await page.evaluate(() => (window as any).__PERFORMANCE_METRICS__);
      expect(performanceData.fps).toBeGreaterThan(20); // Minimum acceptable FPS
    });

    test('should maintain performance during intensive chart interactions', async ({ page }) => {
      // Record baseline performance
      await page.waitForTimeout(1000);
      const baseline = await page.evaluate(() => (window as any).__PERFORMANCE_METRICS__);
      
      // Perform intensive chart interactions
      const canvas = page.locator('#wasm-chart-canvas');
      await canvas.hover();
      
      const interactions = 50;
      const startTime = Date.now();
      
      for (let i = 0; i < interactions; i++) {
        // Alternate between zoom and pan
        if (i % 2 === 0) {
          await page.mouse.wheel(0, -50); // Zoom in
        } else {
          await page.mouse.wheel(0, 50); // Zoom out
        }
        await page.waitForTimeout(20);
      }
      
      const interactionTime = Date.now() - startTime;
      const avgInteractionTime = interactionTime / interactions;
      
      // Each interaction should be processed quickly
      expect(avgInteractionTime).toBeLessThan(50); // 50ms per interaction
      
      // Wait for performance to stabilize
      await page.waitForTimeout(1000);
      
      // Check final performance
      const final = await page.evaluate(() => (window as any).__PERFORMANCE_METRICS__);
      
      // Performance should not degrade significantly
      const fpsRatio = final.fps / baseline.fps;
      expect(fpsRatio).toBeGreaterThan(0.7); // No more than 30% FPS degradation
    });
  });

  test.describe('Memory Management', () => {
    
    test('should properly clean up after symbol changes', async ({ page }) => {
      const symbols = ['BTC-USD', 'ETH-USD', 'ADA-USD'];
      
      // Record initial memory
      const initialMemory = await page.evaluate(() => {
        const perf = (window as any).__PERFORMANCE_METRICS__;
        return perf ? perf.totalMemoryUsage : 0;
      });
      
      // Cycle through symbols multiple times
      for (let cycle = 0; cycle < 5; cycle++) {
        for (const symbol of symbols) {
          await page.selectOption('select[data-testid="symbol-selector"]', symbol);
          await page.waitForTimeout(500);
        }
      }
      
      // Force garbage collection if available
      await page.evaluate(() => {
        if ('gc' in window) {
          (window as any).gc();
        }
      });
      
      await page.waitForTimeout(2000);
      
      // Check final memory
      const finalMemory = await page.evaluate(() => {
        const perf = (window as any).__PERFORMANCE_METRICS__;
        return perf ? perf.totalMemoryUsage : 0;
      });
      
      // Memory growth should be reasonable
      const memoryGrowthMB = (finalMemory - initialMemory) / (1024 * 1024);
      expect(memoryGrowthMB).toBeLessThan(100); // Less than 100MB growth
    });

    test('should handle memory pressure gracefully', async ({ page }) => {
      // Apply memory pressure
      await page.evaluate(() => {
        const arrays = [];
        for (let i = 0; i < 1000; i++) {
          arrays.push(new Array(10000).fill(Math.random()));
        }
        (window as any).__MEMORY_PRESSURE__ = arrays;
      });
      
      // Wait for performance monitoring to detect high memory usage
      await page.waitForTimeout(3000);
      
      // Should trigger memory optimization
      const optimizationTriggered = await page.evaluate(() => {
        return (window as any).__LAST_OPTIMIZATION_APPLIED__?.includes('memory');
      });
      
      expect(optimizationTriggered).toBe(true);
      
      // System should remain functional
      await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
      await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 5000 });
    });

    test('should detect and report memory leaks', async ({ page }) => {
      // Simulate memory leak scenario
      await page.evaluate(() => {
        const leakyObjects = [];
        setInterval(() => {
          leakyObjects.push(new Array(1000).fill(Math.random()));
        }, 100);
        (window as any).__MEMORY_LEAK_SIMULATION__ = leakyObjects;
      });
      
      // Wait for memory monitoring to detect the leak
      await page.waitForTimeout(10000);
      
      // Should detect memory trend as increasing
      const memoryTrend = await page.evaluate(() => {
        const perf = (window as any).__PERFORMANCE_METRICS__;
        return perf ? perf.memoryTrend : 'stable';
      });
      
      expect(memoryTrend).toBe('increasing');
      
      // Should show performance warning
      await expect(page.locator(':text("memory")')).toBeVisible({ timeout: 5000 });
    });
  });

  test.describe('Network Performance', () => {
    
    test('should handle slow network gracefully', async ({ page }) => {
      // Simulate slow network (500ms delay)
      await page.route('**/api/data**', async (route) => {
        await new Promise(resolve => setTimeout(resolve, 500));
        await route.fulfill({
          status: 200,
          body: JSON.stringify(IntegrationDataMockHelper.generateMarketData('BTC-USD', 1000))
        });
      });
      
      const startTime = Date.now();
      
      // Trigger data fetch
      await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
      
      // Should still complete within reasonable time
      await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 3000 });
      
      const totalTime = Date.now() - startTime;
      expect(totalTime).toBeLessThan(2000); // Should complete within 2 seconds
      
      // UI should remain responsive during network delay
      const metrics = await IntegrationTestUtils.getTestMetrics(page);
      expect(metrics.performanceScore).toBeGreaterThan(50);
    });

    test('should batch network requests efficiently', async ({ page }) => {
      let requestCount = 0;
      
      // Monitor network requests
      await page.route('**/api/data**', async (route) => {
        requestCount++;
        await route.fulfill({
          status: 200,
          body: JSON.stringify(IntegrationDataMockHelper.generateMarketData('TEST-USD', 1000))
        });
      });
      
      // Make rapid changes that should be batched
      const symbols = ['BTC-USD', 'ETH-USD', 'ADA-USD'];
      for (const symbol of symbols) {
        await page.selectOption('select[data-testid="symbol-selector"]', symbol);
        await page.waitForTimeout(50); // Very short delay
      }
      
      // Wait for all requests to complete
      await page.waitForTimeout(2000);
      
      // Should have made fewer requests than symbol changes due to debouncing/batching
      expect(requestCount).toBeLessThan(symbols.length);
    });

    test('should maintain performance with poor network conditions', async ({ page }) => {
      // Simulate poor network: high latency and packet loss
      await page.route('**/api/data**', async (route) => {
        // Simulate packet loss (fail 30% of requests)
        if (Math.random() < 0.3) {
          await route.abort();
          return;
        }
        
        // Simulate high latency
        await new Promise(resolve => setTimeout(resolve, 1000));
        await route.fulfill({
          status: 200,
          body: JSON.stringify(IntegrationDataMockHelper.generateMarketData('BTC-USD', 1000))
        });
      });
      
      // Try to use the application normally
      await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
      await page.waitForTimeout(500);
      
      // Should show appropriate network status
      const networkStatus = await page.evaluate(() => {
        const perf = (window as any).__PERFORMANCE_METRICS__;
        return perf ? perf.networkLatency : 0;
      });
      
      expect(networkStatus).toBeGreaterThan(500); // Should detect high latency
      
      // Application should remain functional despite network issues
      await expect(page.locator('#wasm-chart-canvas')).toBeVisible();
      
      // Should show appropriate user feedback
      await expect(page.locator(':text("connection")')).toBeVisible({ timeout: 3000 });
    });
  });

  test.describe('Concurrent Operations', () => {
    
    test('should handle multiple simultaneous operations', async ({ page }) => {
      // Start multiple operations simultaneously
      const operations = [
        // Symbol change
        page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD'),
        
        // Timeframe change  
        page.selectOption('select[data-testid="timeframe-selector"]', '5m'),
        
        // Chart interaction
        (async () => {
          const canvas = page.locator('#wasm-chart-canvas');
          await canvas.hover();
          await page.mouse.wheel(0, -100);
        })(),
        
        // Performance monitoring action
        page.evaluate(() => {
          if ((window as any).__TRIGGER_PERFORMANCE_CHECK__) {
            (window as any).__TRIGGER_PERFORMANCE_CHECK__();
          }
        })
      ];
      
      // Execute all operations concurrently
      const startTime = Date.now();
      await Promise.all(operations);
      const concurrentTime = Date.now() - startTime;
      
      // Should handle concurrent operations quickly
      expect(concurrentTime).toBeLessThan(2000);
      
      // Wait for all operations to settle
      await page.waitForTimeout(1000);
      
      // System should be in consistent state
      await IntegrationTestUtils.verifySystemConsistency(page, {
        symbol: 'BTC-USD',
        timeframe: '5m'
      });
      
      // Performance should be acceptable
      const metrics = await IntegrationTestUtils.getTestMetrics(page);
      expect(metrics.performanceScore).toBeGreaterThan(60);
    });

    test('should prioritize user interactions over background tasks', async ({ page }) => {
      // Start background data fetching
      await IntegrationDataMockHelper.mockRealTimeDataStream(page, 'BTC-USD', 5000);
      
      // Measure user interaction responsiveness during background activity
      const canvas = page.locator('#wasm-chart-canvas');
      await canvas.hover();
      
      const interactionTimes = [];
      
      for (let i = 0; i < 10; i++) {
        const startTime = Date.now();
        await page.mouse.wheel(0, -25);
        
        // Wait for interaction to be processed
        await page.waitForTimeout(50);
        
        const interactionTime = Date.now() - startTime;
        interactionTimes.push(interactionTime);
      }
      
      // User interactions should remain responsive
      const avgInteractionTime = interactionTimes.reduce((a, b) => a + b) / interactionTimes.length;
      expect(avgInteractionTime).toBeLessThan(100); // 100ms average response time
      
      // No interaction should take extremely long
      const maxInteractionTime = Math.max(...interactionTimes);
      expect(maxInteractionTime).toBeLessThan(300); // 300ms max response time
    });
  });

  test.describe('Performance Regression Detection', () => {
    
    test('should not regress from baseline performance', async ({ page }) => {
      // This test would compare against known performance baselines
      // In a real implementation, you would store baseline metrics
      
      const currentMetrics = await IntegrationTestUtils.getTestMetrics(page);
      
      // Example baseline expectations (these would be stored/loaded from a file)
      const baselineExpectations = {
        minPerformanceScore: 70,
        maxInitializationTime: 8000,
        maxMemoryUsage: 300 * 1024 * 1024, // 300MB
        minFps: 30
      };
      
      expect(currentMetrics.performanceScore).toBeGreaterThan(baselineExpectations.minPerformanceScore);
      
      const performanceData = await page.evaluate(() => (window as any).__PERFORMANCE_METRICS__);
      expect(performanceData.totalMemoryUsage).toBeLessThan(baselineExpectations.maxMemoryUsage);
      expect(performanceData.fps).toBeGreaterThan(baselineExpectations.minFps);
    });

    test('should scale performance linearly with data size', async ({ page }) => {
      const dataSizes = [1000, 5000, 10000];
      const processingTimes = [];
      
      for (const dataSize of dataSizes) {
        // Mock specific data size
        await IntegrationDataMockHelper.mockServerResponse(page, 
          IntegrationDataMockHelper.generateMarketData('TEST-USD', dataSize)
        );
        
        const startTime = Date.now();
        
        // Trigger data processing
        await page.selectOption('select[data-testid="symbol-selector"]', 'TEST-USD');
        await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 10000 });
        
        const processingTime = Date.now() - startTime;
        processingTimes.push({ dataSize, processingTime });
        
        // Reset for next test
        await page.waitForTimeout(500);
      }
      
      // Verify linear scaling (processing time should scale reasonably with data size)
      const timeRatio1 = processingTimes[1].processingTime / processingTimes[0].processingTime;
      const timeRatio2 = processingTimes[2].processingTime / processingTimes[1].processingTime;
      
      // Ratios should be reasonable (not exponential growth)
      expect(timeRatio1).toBeLessThan(8); // 5x data should not take more than 8x time
      expect(timeRatio2).toBeLessThan(3); // 2x data should not take more than 3x time
    });
  });
});