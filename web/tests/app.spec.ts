import { test, expect, Page } from '@playwright/test';

// Test data constants
const TEST_TOPICS = {
  BTC: 'BTC-usd',
  SENSOR: 'sensor_data'
};

const TEST_TIME_RANGES = {
  VALID: { start: 1745322750, end: 1745691150 },
  INVALID: { start: 9999999999, end: 9999999990 }
};

test.describe('Graph Visualization App', () => {
  
  test.beforeEach(async ({ page }) => {
    // Enable WebGPU if available
    await page.goto('/');
  });

  test('should load the application successfully', async ({ page }) => {
    await expect(page).toHaveTitle(/Graph/i);
    
    // Check for main app container
    await expect(page.locator('#root')).toBeVisible();
  });

  test('should detect WebGPU support', async ({ page, browserName }) => {
    // Navigate to app page to trigger WASM loading
    await page.goto(`/app`);
    
    // Check if WebGPU is available in this browser
    const webgpuSupported = await page.evaluate(() => {
      return 'gpu' in navigator;
    });

    if (webgpuSupported) {
      console.log(`WebGPU supported in ${browserName}`);
      
      // Wait for WASM to load and initialize
      await page.waitForTimeout(3000);
      
      // Check for canvas element (should be created by WASM)
      await expect(page.locator('#new-api-canvas')).toBeVisible({ timeout: 10000 });
    } else {
      console.log(`WebGPU not supported in ${browserName}, testing fallback behavior`);
      
      // Should show error message or fallback UI
      // This depends on how your app handles WebGPU unavailability
    }
  });

  test('should load WASM module successfully', async ({ page }) => {
    await page.goto(`/app`);
    
    // Wait for canvas to appear (indicates WASM loaded)
    await expect(page.locator('#new-api-canvas')).toBeVisible({ timeout: 15000 });
    
    // Wait for loading overlay to disappear
    await page.waitForSelector('[data-testid="loading-overlay"]', { state: 'detached', timeout: 15000 }).catch(() => {
      // Fallback: check if loading text is gone
      return page.waitForFunction(() => !document.body.textContent?.includes('Loading Chart Engine'), { timeout: 15000 });
    });
    
    // Canvas should be properly sized
    const canvas = page.locator('#new-api-canvas');
    const box = await canvas.boundingBox();
    expect(box?.width).toBeGreaterThan(200);
    expect(box?.height).toBeGreaterThan(100);
  });

  test('should render chart with valid data', async ({ page }) => {
    await page.goto(`/app`);
    
    // Wait for WASM and initial render
    await page.waitForTimeout(5000);
    
    // Should have canvas element
    const canvas = page.locator('#new-api-canvas');
    await expect(canvas).toBeVisible();
    
    // Canvas should have reasonable dimensions
    const canvasBox = await canvas.boundingBox();
    expect(canvasBox?.width).toBeGreaterThan(300);
    expect(canvasBox?.height).toBeGreaterThan(200);
  });

  test('should handle zoom interactions', async ({ page }) => {
    await page.goto(`/app?topic=${TEST_TOPICS.BTC}&start=${TEST_TIME_RANGES.VALID.start}&end=${TEST_TIME_RANGES.VALID.end}`);
    
    // Wait for chart to load
    await page.waitForTimeout(5000);
    
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible();
    
    // Get initial viewport state
    const initialState = await page.evaluate(() => {
      return {
        scrollX: window.scrollX,
        scrollY: window.scrollY
      };
    });
    
    // Simulate zoom (wheel event on canvas)
    await canvas.hover();
    await page.mouse.wheel(0, -100); // Zoom in
    
    // Wait for zoom to process
    await page.waitForTimeout(1000);
    
    // Zoom out
    await page.mouse.wheel(0, 100);
    await page.waitForTimeout(1000);
    
    // Test should not crash during zoom operations
    await expect(canvas).toBeVisible();
  });

  test('should handle pan interactions', async ({ page }) => {
    await page.goto(`/app?topic=${TEST_TOPICS.BTC}&start=${TEST_TIME_RANGES.VALID.start}&end=${TEST_TIME_RANGES.VALID.end}`);
    
    // Wait for chart to load
    await page.waitForTimeout(5000);
    
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible();
    
    // Simulate pan (drag on canvas)
    const canvasBox = await canvas.boundingBox();
    if (canvasBox) {
      const centerX = canvasBox.x + canvasBox.width / 2;
      const centerY = canvasBox.y + canvasBox.height / 2;
      
      // Pan left
      await page.mouse.move(centerX, centerY);
      await page.mouse.down();
      await page.mouse.move(centerX - 100, centerY);
      await page.mouse.up();
      
      await page.waitForTimeout(1000);
      
      // Pan right
      await page.mouse.move(centerX - 100, centerY);
      await page.mouse.down();
      await page.mouse.move(centerX + 100, centerY);
      await page.mouse.up();
      
      await page.waitForTimeout(1000);
    }
    
    // Should not crash during pan operations
    await expect(canvas).toBeVisible();
  });

  test('should handle invalid data gracefully', async ({ page }) => {
    // Test with invalid time range
    await page.goto(`/app?topic=${TEST_TOPICS.BTC}&start=${TEST_TIME_RANGES.INVALID.start}&end=${TEST_TIME_RANGES.INVALID.end}`);
    
    // Wait for potential error handling
    await page.waitForTimeout(3000);
    
    // Should either show error message or empty chart, but not crash
    const hasCanvas = await page.locator('canvas').isVisible();
    const hasError = await page.locator('[data-testid="error"]').isVisible();
    
    // One of these should be true (either shows chart or error)
    expect(hasCanvas || hasError).toBe(true);
  });

  test('should handle network failures gracefully', async ({ page }) => {
    // First load the app normally
    await page.goto(`/app`);
    
    // Wait for app to load
    await expect(page.locator('#new-api-canvas')).toBeVisible({ timeout: 15000 });
    
    // Then block only API requests, not the app itself
    await page.route('**/api/**', route => route.abort());
    
    // App should still be functional even if API calls fail
    await page.waitForTimeout(2000);
    
    // Should not show complete crash
    await expect(page.locator('#root')).toBeVisible();
  });

  test('should maintain performance during extended use', async ({ page }) => {
    await page.goto(`/app?topic=${TEST_TOPICS.BTC}&start=${TEST_TIME_RANGES.VALID.start}&end=${TEST_TIME_RANGES.VALID.end}`);
    
    // Wait for initial load
    await page.waitForTimeout(5000);
    
    const canvas = page.locator('canvas');
    await expect(canvas).toBeVisible();
    
    // Get initial memory usage
    const initialMemory = await page.evaluate(() => {
      return (performance as any).memory?.usedJSHeapSize || 0;
    });
    
    // Perform multiple interactions
    for (let i = 0; i < 10; i++) {
      await canvas.hover();
      await page.mouse.wheel(0, -50); // Zoom in
      await page.waitForTimeout(100);
      await page.mouse.wheel(0, 50);  // Zoom out
      await page.waitForTimeout(100);
    }
    
    // Check memory usage hasn't grown excessively
    const finalMemory = await page.evaluate(() => {
      return (performance as any).memory?.usedJSHeapSize || 0;
    });
    
    if (initialMemory > 0 && finalMemory > 0) {
      const memoryGrowth = finalMemory - initialMemory;
      const maxAcceptableGrowth = initialMemory * 2; // Allow 100% growth max
      
      expect(memoryGrowth).toBeLessThan(maxAcceptableGrowth);
    }
  });
});

// Browser-specific tests
test.describe('Browser Compatibility', () => {
  
  test('should work in Chromium with WebGPU', async ({ page, browserName }) => {
    test.skip(browserName !== 'chromium', 'WebGPU testing only in Chromium');
    
    await page.goto(`/app?topic=${TEST_TOPICS.BTC}&start=${TEST_TIME_RANGES.VALID.start}&end=${TEST_TIME_RANGES.VALID.end}`);
    
    // Check WebGPU is available and working
    const webgpuWorking = await page.evaluate(async () => {
      if (!('gpu' in navigator)) return false;
      
      try {
        const adapter = await navigator.gpu.requestAdapter();
        return adapter !== null;
      } catch {
        return false;
      }
    });
    
    expect(webgpuWorking).toBe(true);
    
    // Chart should render
    await expect(page.locator('canvas')).toBeVisible({ timeout: 10000 });
  });
  
  test('should handle WebGPU unavailability', async ({ page, browserName }) => {
    // Disable WebGPU for this test
    await page.addInitScript(() => {
      delete (navigator as any).gpu;
    });
    
    await page.goto(`/app?topic=${TEST_TOPICS.BTC}&start=${TEST_TIME_RANGES.VALID.start}&end=${TEST_TIME_RANGES.VALID.end}`);
    
    // App should still load, potentially with fallback rendering
    await page.waitForTimeout(3000);
    
    // Should not crash completely
    const hasContent = await page.locator('#root').isVisible();
    expect(hasContent).toBe(true);
  });
});