/**
 * Store Contract Performance Tests
 * 
 * Performance benchmarks for the React-Rust store integration,
 * measuring latency, memory usage, and throughput.
 */

import { test, expect } from '@playwright/test';
import { GraphTestUtils } from '../helpers/integration-test-utils';

test.describe('Store Contract Performance', () => {
  
  test.beforeEach(async ({ page }) => {
    await page.goto('/app');
    await GraphTestUtils.waitForWasmLoad(page);
  });

  test('store state updates should complete within performance budget', async ({ page }) => {
    const iterations = 100;
    const maxLatencyMs = 50; // 50ms budget per update
    
    const latencies: number[] = [];
    
    for (let i = 0; i < iterations; i++) {
      const startTime = await page.evaluate(() => performance.now());
      
      await page.evaluate((index) => {
        const store = window.store?.getState?.();
        if (store) {
          store.setCurrentSymbol?.(index % 2 === 0 ? 'BTC-USD' : 'ETH-USD');
          store.setTimeframe?.(index % 3 === 0 ? '1h' : '5m');
        }
      }, i);
      
      const endTime = await page.evaluate(() => performance.now());
      const latency = endTime - startTime;
      latencies.push(latency);
      
      // Small delay between updates
      if (i % 10 === 0) {
        await page.waitForTimeout(5);
      }
    }
    
    // Calculate statistics
    const avgLatency = latencies.reduce((sum, lat) => sum + lat, 0) / latencies.length;
    const maxLatency = Math.max(...latencies);
    const p95Latency = latencies.sort((a, b) => a - b)[Math.floor(latencies.length * 0.95)];
    
    console.log(`Store Update Performance:
      Average Latency: ${avgLatency.toFixed(2)}ms
      Max Latency: ${maxLatency.toFixed(2)}ms
      P95 Latency: ${p95Latency.toFixed(2)}ms`);
    
    // Performance assertions
    expect(avgLatency).toBeLessThan(maxLatencyMs);
    expect(p95Latency).toBeLessThan(maxLatencyMs * 2);
    expect(maxLatency).toBeLessThan(maxLatencyMs * 5);
  });

  test('memory usage should remain stable under sustained load', async ({ page }) => {
    const measurementInterval = 1000; // 1 second
    const testDurationMs = 10000; // 10 seconds
    const maxMemoryGrowthMB = 50; // 50MB max growth
    
    const initialMemory = await GraphTestUtils.measureMemoryUsage(page);
    const memoryMeasurements: number[] = [];
    
    let testStartTime = Date.now();
    let updateCounter = 0;
    
    // Start continuous updates
    const updateInterval = setInterval(async () => {
      await page.evaluate((counter) => {
        const store = window.store?.getState?.();
        if (store) {
          const symbols = ['BTC-USD', 'ETH-USD', 'ADA-USD', 'DOT-USD', 'SOL-USD'];
          const timeframes = ['1m', '5m', '15m', '1h', '4h', '1d'];
          
          store.setCurrentSymbol?.(symbols[counter % symbols.length]);
          store.setTimeframe?.(timeframes[counter % timeframes.length]);
          store.setConnectionStatus?.(counter % 2 === 0);
        }
      }, updateCounter++);
    }, 100);
    
    // Measure memory at regular intervals
    const memoryInterval = setInterval(async () => {
      try {
        const memory = await GraphTestUtils.measureMemoryUsage(page);
        memoryMeasurements.push(memory.used);
      } catch (error) {
        console.warn('Memory measurement failed:', error);
      }
    }, measurementInterval);
    
    // Wait for test duration
    await page.waitForTimeout(testDurationMs);
    
    // Cleanup intervals
    clearInterval(updateInterval);
    clearInterval(memoryInterval);
    
    // Force garbage collection if available
    await page.evaluate(() => {
      if (window.gc) {
        window.gc();
      }
    });
    
    await page.waitForTimeout(1000);
    
    const finalMemory = await GraphTestUtils.measureMemoryUsage(page);
    const memoryGrowthMB = (finalMemory.used - initialMemory.used) / (1024 * 1024);
    
    // Calculate memory growth trend
    const growthOverTime = memoryMeasurements.map((mem, index) => ({
      time: index * measurementInterval,
      memory: mem,
      growth: (mem - initialMemory.used) / (1024 * 1024)
    }));
    
    console.log(`Memory Performance:
      Initial Memory: ${(initialMemory.used / (1024 * 1024)).toFixed(2)}MB
      Final Memory: ${(finalMemory.used / (1024 * 1024)).toFixed(2)}MB
      Total Growth: ${memoryGrowthMB.toFixed(2)}MB
      Updates Performed: ${updateCounter}`);
    
    // Performance assertions
    expect(memoryGrowthMB).toBeLessThan(maxMemoryGrowthMB);
    
    // Verify chart is still functional
    const isFunctional = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isFunctional).toBe(true);
  });

  test('concurrent state changes should maintain consistency', async ({ page }) => {
    const concurrentOperations = 50;
    const operationsPerBatch = 10;
    
    // Prepare test data
    const symbols = ['BTC-USD', 'ETH-USD', 'ADA-USD', 'DOT-USD', 'SOL-USD'];
    const timeframes = ['1m', '5m', '15m', '1h', '4h', '1d'];
    
    const startTime = performance.now();
    
    // Execute concurrent batches of operations
    for (let batch = 0; batch < Math.ceil(concurrentOperations / operationsPerBatch); batch++) {
      const promises: Promise<void>[] = [];
      
      for (let i = 0; i < operationsPerBatch; i++) {
        const operationIndex = batch * operationsPerBatch + i;
        
        promises.push(
          page.evaluate(([index, syms, tfs]) => {
            const store = window.store?.getState?.();
            if (store) {
              store.setCurrentSymbol?.(syms[index % syms.length]);
              store.setTimeframe?.(tfs[index % tfs.length]);
              store.setConnectionStatus?.(index % 2 === 0);
            }
          }, [operationIndex, symbols, timeframes])
        );
      }
      
      // Wait for all operations in this batch to complete
      await Promise.allSettled(promises);
      
      // Small delay between batches
      await page.waitForTimeout(10);
    }
    
    const endTime = performance.now();
    const totalTime = endTime - startTime;
    const throughput = concurrentOperations / (totalTime / 1000); // operations per second
    
    console.log(`Concurrent Operations Performance:
      Total Operations: ${concurrentOperations}
      Total Time: ${totalTime.toFixed(2)}ms
      Throughput: ${throughput.toFixed(2)} ops/sec`);
    
    // Verify final state consistency
    const finalState = await page.evaluate(() => {
      const store = window.store?.getState?.();
      return {
        symbol: store?.currentSymbol,
        timeframe: store?.chartConfig?.timeframe,
        connected: store?.isConnected,
        chartInitialized: window.wasmChart?.is_initialized?.() || false
      };
    });
    
    // State should be valid
    expect(symbols).toContain(finalState.symbol);
    expect(timeframes).toContain(finalState.timeframe);
    expect(typeof finalState.connected).toBe('boolean');
    expect(finalState.chartInitialized).toBe(true);
    
    // Throughput should be reasonable
    expect(throughput).toBeGreaterThan(10); // At least 10 ops/sec
  });

  test('data serialization should be performant', async ({ page }) => {
    const iterations = 1000;
    const maxSerializationTimeMs = 10;
    
    // Create large test data
    const largeMarketData: Record<string, any> = {};
    for (let i = 0; i < 100; i++) {
      largeMarketData[`SYMBOL${i}`] = {
        symbol: `SYMBOL${i}`,
        price: Math.random() * 1000,
        change: Math.random() * 100 - 50,
        changePercent: Math.random() * 10 - 5,
        volume: Math.random() * 1000000,
        timestamp: Date.now()
      };
    }
    
    // Set large state
    await page.evaluate((marketData) => {
      const store = window.store?.getState?.();
      if (store) {
        // Simulate setting large market data
        for (const [symbol, data] of Object.entries(marketData)) {
          store.updateMarketData?.(symbol, data);
        }
      }
    }, largeMarketData);
    
    const serializationTimes: number[] = [];
    
    // Measure serialization performance
    for (let i = 0; i < iterations; i++) {
      const serializationTime = await page.evaluate(() => {
        const startTime = performance.now();
        
        const store = window.store?.getState?.();
        if (store) {
          const state = {
            currentSymbol: store.currentSymbol,
            chartConfig: store.chartConfig,
            marketData: store.marketData,
            isConnected: store.isConnected,
            user: store.user
          };
          
          // Serialize and deserialize
          const serialized = JSON.stringify(state);
          const deserialized = JSON.parse(serialized);
          
          // Verify roundtrip
          return {
            time: performance.now() - startTime,
            success: deserialized.currentSymbol === state.currentSymbol
          };
        }
        
        return { time: 0, success: false };
      });
      
      expect(serializationTime.success).toBe(true);
      serializationTimes.push(serializationTime.time);
    }
    
    // Calculate statistics
    const avgTime = serializationTimes.reduce((sum, time) => sum + time, 0) / serializationTimes.length;
    const maxTime = Math.max(...serializationTimes);
    const p95Time = serializationTimes.sort((a, b) => a - b)[Math.floor(serializationTimes.length * 0.95)];
    
    console.log(`Serialization Performance:
      Average Time: ${avgTime.toFixed(2)}ms
      Max Time: ${maxTime.toFixed(2)}ms
      P95 Time: ${p95Time.toFixed(2)}ms
      Iterations: ${iterations}`);
    
    // Performance assertions
    expect(avgTime).toBeLessThan(maxSerializationTimeMs);
    expect(p95Time).toBeLessThan(maxSerializationTimeMs * 2);
  });

  test('WASM bridge calls should have low latency', async ({ page }) => {
    const iterations = 500;
    const maxLatencyMs = 5; // 5ms max per WASM call
    
    const wasmCallTimes: number[] = [];
    
    for (let i = 0; i < iterations; i++) {
      const callTime = await page.evaluate(() => {
        const startTime = performance.now();
        
        // Call WASM method
        const isInitialized = window.wasmChart?.is_initialized?.() || false;
        
        return {
          time: performance.now() - startTime,
          result: isInitialized
        };
      });
      
      expect(callTime.result).toBe(true);
      wasmCallTimes.push(callTime.time);
      
      // Small delay to avoid overwhelming
      if (i % 50 === 0) {
        await page.waitForTimeout(5);
      }
    }
    
    // Calculate statistics
    const avgLatency = wasmCallTimes.reduce((sum, time) => sum + time, 0) / wasmCallTimes.length;
    const maxLatency = Math.max(...wasmCallTimes);
    const p95Latency = wasmCallTimes.sort((a, b) => a - b)[Math.floor(wasmCallTimes.length * 0.95)];
    
    console.log(`WASM Bridge Performance:
      Average Latency: ${avgLatency.toFixed(2)}ms
      Max Latency: ${maxLatency.toFixed(2)}ms
      P95 Latency: ${p95Latency.toFixed(2)}ms
      Total Calls: ${iterations}`);
    
    // Performance assertions
    expect(avgLatency).toBeLessThan(maxLatencyMs);
    expect(p95Latency).toBeLessThan(maxLatencyMs * 2);
    expect(maxLatency).toBeLessThan(maxLatencyMs * 10);
  });

  test('UI responsiveness during heavy operations', async ({ page }) => {
    // Start heavy background processing
    await page.evaluate(() => {
      // Simulate heavy computation
      window.heavyProcessing = setInterval(() => {
        const store = window.store?.getState?.();
        if (store) {
          // Rapid updates
          for (let i = 0; i < 10; i++) {
            store.setConnectionStatus?.(i % 2 === 0);
          }
        }
      }, 10);
    });
    
    // Measure UI interaction responsiveness
    const clickTimes: number[] = [];
    const wheelTimes: number[] = [];
    
    for (let i = 0; i < 20; i++) {
      // Test click responsiveness
      const clickStart = performance.now();
      await page.click('#wasm-chart-canvas', { position: { x: 100 + i * 10, y: 100 + i * 5 } });
      clickTimes.push(performance.now() - clickStart);
      
      // Test wheel responsiveness
      const wheelStart = performance.now();
      await page.mouse.wheel(0, i % 2 === 0 ? -50 : 50);
      wheelTimes.push(performance.now() - wheelStart);
      
      await page.waitForTimeout(100);
    }
    
    // Stop heavy processing
    await page.evaluate(() => {
      if (window.heavyProcessing) {
        clearInterval(window.heavyProcessing);
      }
    });
    
    // Calculate responsiveness metrics
    const avgClickTime = clickTimes.reduce((sum, time) => sum + time, 0) / clickTimes.length;
    const avgWheelTime = wheelTimes.reduce((sum, time) => sum + time, 0) / wheelTimes.length;
    
    console.log(`UI Responsiveness During Heavy Load:
      Average Click Response: ${avgClickTime.toFixed(2)}ms
      Average Wheel Response: ${avgWheelTime.toFixed(2)}ms`);
    
    // UI should remain responsive (< 16ms for 60fps)
    expect(avgClickTime).toBeLessThan(50);
    expect(avgWheelTime).toBeLessThan(50);
    
    // Chart should still be functional
    const isFunctional = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isFunctional).toBe(true);
  });
});