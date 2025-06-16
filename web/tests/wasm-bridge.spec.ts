/**
 * WASM Bridge Integration Tests
 * 
 * Tests for the React-Rust WASM bridge functionality,
 * including chart initialization, state synchronization, and error handling.
 */

import { test, expect } from '@playwright/test';
import { GraphTestUtils } from './helpers/integration-test-utils';

test.describe('WASM Bridge Integration', () => {
  
  test.beforeEach(async ({ page }) => {
    await page.goto('/app');
    // Add global error tracking
    await page.addInitScript(() => {
      window.wasmErrors = [];
      window.addEventListener('error', (e) => {
        window.wasmErrors.push({
          message: e.message,
          filename: e.filename,
          lineno: e.lineno,
          colno: e.colno,
          timestamp: Date.now()
        });
      });
    });
  });

  test('should initialize WASM chart successfully', async ({ page }) => {
    // Wait for WASM initialization
    await GraphTestUtils.waitForWasmLoad(page);
    
    // Verify chart is initialized
    const isInitialized = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isInitialized).toBe(true);
    
    // Verify no initialization errors occurred
    const errors = await page.evaluate(() => window.wasmErrors || []);
    expect(errors).toHaveLength(0);
  });

  test('should handle chart resize correctly', async ({ page }) => {
    await GraphTestUtils.waitForWasmLoad(page);
    
    // Get canvas element
    const canvas = page.locator('#wasm-chart-canvas');
    await expect(canvas).toBeVisible();
    
    // Change viewport size
    await page.setViewportSize({ width: 1200, height: 800 });
    
    // Wait for resize to be processed
    await page.waitForTimeout(200);
    
    // Verify chart is still functional
    const isStillInitialized = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isStillInitialized).toBe(true);
  });

  test('should handle mouse wheel events', async ({ page }) => {
    await GraphTestUtils.waitForWasmLoad(page);
    
    const canvas = page.locator('#wasm-chart-canvas');
    
    // Test mouse wheel interaction
    await canvas.hover();
    await page.mouse.wheel(0, -100); // Scroll up (zoom in)
    
    // Verify chart is still responsive
    const isResponsive = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isResponsive).toBe(true);
    
    // Test multiple wheel events
    for (let i = 0; i < 5; i++) {
      await page.mouse.wheel(0, 50);
      await page.waitForTimeout(10);
    }
    
    // Verify no errors from rapid interactions
    const errors = await page.evaluate(() => window.wasmErrors || []);
    expect(errors).toHaveLength(0);
  });

  test('should handle mouse click events', async ({ page }) => {
    await GraphTestUtils.waitForWasmLoad(page);
    
    const canvas = page.locator('#wasm-chart-canvas');
    
    // Test single click
    await canvas.click({ position: { x: 100, y: 100 } });
    
    // Test double click
    await canvas.dblclick({ position: { x: 200, y: 200 } });
    
    // Test click and drag
    await canvas.click({ position: { x: 150, y: 150 } });
    await page.mouse.move(250, 250);
    
    // Verify chart remains functional
    const isFunctional = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isFunctional).toBe(true);
  });

  test('should handle rapid state updates without memory leaks', async ({ page }) => {
    await GraphTestUtils.waitForWasmLoad(page);
    
    const initialMemory = await GraphTestUtils.measureMemoryUsage(page);
    
    // Perform many rapid state updates
    for (let i = 0; i < 50; i++) {
      await page.evaluate((index) => {
        // Simulate rapid store updates
        const store = window.store?.getState?.();
        if (store) {
          store.setCurrentSymbol?.(index % 2 === 0 ? 'BTC-USD' : 'ETH-USD');
          store.setTimeframe?.(index % 3 === 0 ? '1h' : '5m');
          store.setConnectionStatus?.(index % 2 === 0);
        }
      }, i);
      
      if (i % 10 === 0) {
        await page.waitForTimeout(10);
      }
    }
    
    // Force garbage collection if available
    await page.evaluate(() => {
      if (window.gc) {
        window.gc();
      }
    });
    
    await page.waitForTimeout(100);
    
    const finalMemory = await GraphTestUtils.measureMemoryUsage(page);
    const memoryGrowth = finalMemory.used - initialMemory.used;
    const growthPercentage = (memoryGrowth / initialMemory.used) * 100;
    
    // Memory growth should be reasonable
    expect(growthPercentage).toBeLessThan(100);
    
    // Chart should still be functional
    const isFunctional = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isFunctional).toBe(true);
  });

  test('should recover from WASM module loading failures', async ({ page }) => {
    // Simulate WASM loading failure by blocking the module
    await page.route('**/pkg/**', route => {
      route.abort();
    });
    
    await page.goto('/app');
    
    // Wait for error state
    await page.waitForSelector('[data-testid="error-overlay"]', { timeout: 10000 });
    
    // Verify error overlay is shown
    const errorOverlay = page.locator('[data-testid="error-overlay"]');
    await expect(errorOverlay).toBeVisible();
    
    // Check for retry button
    const retryButton = page.locator('[data-testid="retry-button"]');
    if (await retryButton.isVisible()) {
      expect(retryButton).toBeVisible();
    }
  });

  test('should handle WebGPU initialization failures gracefully', async ({ page }) => {
    // Mock WebGPU to return null (unsupported)
    await page.addInitScript(() => {
      Object.defineProperty(navigator, 'gpu', {
        value: null,
        writable: false
      });
    });
    
    await page.goto('/app');
    
    // Should either show error or fallback gracefully
    await page.waitForTimeout(5000);
    
    // Check if error handling worked
    const hasErrorState = await page.evaluate(() => {
      return document.querySelector('[data-testid="error-overlay"]') !== null;
    });
    
    const hasCanvas = await page.evaluate(() => {
      return document.querySelector('#wasm-chart-canvas') !== null;
    });
    
    // Should either show error or have canvas (depending on fallback)
    expect(hasErrorState || hasCanvas).toBe(true);
  });

  test('should maintain chart state across re-initialization', async ({ page }) => {
    await GraphTestUtils.waitForWasmLoad(page);
    
    // Set initial state
    await page.evaluate(() => {
      const store = window.store?.getState?.();
      if (store) {
        store.setCurrentSymbol?.('ETH-USD');
        store.setTimeframe?.('4h');
      }
    });
    
    await page.waitForTimeout(200);
    
    // Force re-initialization by resetting
    const resetResult = await page.evaluate(async () => {
      if (window.wasmChartAPI?.reset) {
        return await window.wasmChartAPI.reset();
      }
      return false;
    });
    
    if (resetResult) {
      await page.waitForTimeout(1000);
      
      // Verify state is maintained or reset properly
      const state = await page.evaluate(() => {
        const store = window.store?.getState?.();
        return {
          symbol: store?.currentSymbol,
          timeframe: store?.chartConfig?.timeframe,
          initialized: window.wasmChart?.is_initialized?.() || false
        };
      });
      
      expect(state.initialized).toBe(true);
      expect(state.symbol).toBeTruthy();
      expect(['1m', '5m', '15m', '1h', '4h', '1d']).toContain(state.timeframe);
    }
  });

  test('should handle canvas context loss', async ({ page }) => {
    await GraphTestUtils.waitForWasmLoad(page);
    
    // Simulate context loss
    await page.evaluate(() => {
      const canvas = document.getElementById('wasm-chart-canvas') as HTMLCanvasElement;
      if (canvas) {
        // Simulate WebGL context loss
        const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
        if (gl && gl.getExtension('WEBGL_lose_context')) {
          gl.getExtension('WEBGL_lose_context')!.loseContext();
        }
      }
    });
    
    await page.waitForTimeout(1000);
    
    // Check if application handles context loss gracefully
    const isHandled = await page.evaluate(() => {
      // Should either recover or show appropriate error
      const hasError = document.querySelector('[data-testid="error-overlay"]') !== null;
      const isInitialized = window.wasmChart?.is_initialized?.() || false;
      return hasError || isInitialized;
    });
    
    expect(isHandled).toBe(true);
  });

  test('should handle performance monitoring correctly', async ({ page }) => {
    await GraphTestUtils.waitForWasmLoad(page);
    
    // Verify performance overlay is visible
    const performanceOverlay = page.locator('.absolute.top-4.right-4').first();
    await expect(performanceOverlay).toBeVisible({ timeout: 5000 });
    
    // Check for FPS display
    const fpsDisplay = performanceOverlay.locator('text=/FPS/');
    await expect(fpsDisplay).toBeVisible();
    
    // Verify metrics are updating
    await page.waitForTimeout(2000);
    
    const metricsText = await performanceOverlay.textContent();
    expect(metricsText).toMatch(/FPS:\s*\d+/);
    expect(metricsText).toMatch(/Updates:\s*\d+/);
  });

  test('should handle debug mode correctly', async ({ page }) => {
    // Enable debug mode
    await page.goto('/app?debug=true');
    await GraphTestUtils.waitForWasmLoad(page);
    
    // Check for debug panel
    const debugPanel = page.locator('text=Debug Panel');
    await expect(debugPanel).toBeVisible({ timeout: 5000 });
    
    // Test debug actions
    const forceUpdateButton = page.locator('[data-testid="force-update-button"]');
    if (await forceUpdateButton.isVisible()) {
      await forceUpdateButton.click();
      await page.waitForTimeout(100);
    }
    
    const getStateButton = page.locator('[data-testid="get-state-button"]');
    if (await getStateButton.isVisible()) {
      await getStateButton.click();
      await page.waitForTimeout(100);
    }
    
    // Verify chart remains functional after debug actions
    const isFunctional = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isFunctional).toBe(true);
  });

  test('should handle concurrent WASM operations', async ({ page }) => {
    await GraphTestUtils.waitForWasmLoad(page);
    
    // Perform multiple concurrent operations
    const operations = [
      page.evaluate(() => window.wasmChart?.is_initialized?.()),
      page.evaluate(() => {
        const store = window.store?.getState?.();
        if (store) store.setCurrentSymbol?.('BTC-USD');
      }),
      page.evaluate(() => {
        const store = window.store?.getState?.();
        if (store) store.setTimeframe?.('1h');
      }),
      page.mouse.wheel(0, -50),
      page.mouse.click(100, 100)
    ];
    
    // Execute all operations concurrently
    const results = await Promise.allSettled(operations);
    
    // Most operations should succeed
    const successCount = results.filter(r => r.status === 'fulfilled').length;
    expect(successCount).toBeGreaterThan(2);
    
    // Chart should remain functional
    const isFunctional = await page.evaluate(() => {
      return window.wasmChart?.is_initialized?.() || false;
    });
    
    expect(isFunctional).toBe(true);
  });
});