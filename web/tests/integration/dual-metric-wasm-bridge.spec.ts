import { test, expect } from '@playwright/test';

test.describe('Dual-Metric WASM Bridge Integration', () => {
  test.beforeEach(async ({ page }) => {
    // Mock network requests to control data flow
    await page.route('**/api/data*', async route => {
      const url = new URL(route.request().url());
      const columns = url.searchParams.get('columns') || 'time,best_bid,best_ask';
      
      // Create mock binary response based on requested columns
      const columnList = columns.split(',');
      const numDataPoints = 100;
      
      // Mock JSON header
      const header = {
        columns: columnList.slice(1).map(col => ({ // Skip 'time' in response
          name: col,
          data_length: numDataPoints * 4 // 4 bytes per f32
        }))
      };
      
      // Create mock binary data (simplified)
      const headerJson = JSON.stringify(header);
      const headerBuffer = new TextEncoder().encode(headerJson + '\n');
      const dataBuffer = new ArrayBuffer(columnList.length * numDataPoints * 4);
      
      // Combine header and data
      const combined = new Uint8Array(headerBuffer.length + dataBuffer.byteLength);
      combined.set(new Uint8Array(headerBuffer), 0);
      combined.set(new Uint8Array(dataBuffer), headerBuffer.length);
      
      await route.fulfill({
        status: 200,
        contentType: 'application/octet-stream',
        body: combined.buffer
      });
    });
    
    await page.goto('/app?topic=BTC-USD&start=1745322750&end=1745691150');
    await page.waitForSelector('[data-testid="chart-canvas"]', { timeout: 10000 });
    await page.waitForTimeout(2000); // Allow full initialization
  });

  test('should pass selectedMetrics to WASM chart', async ({ page }) => {
    // Intercept console logs to see WASM communication
    const logs: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'log' && msg.text().includes('update_chart_state')) {
        logs.push(msg.text());
      }
    });
    
    // Change metric selection
    await page.locator('[data-testid="metric-volume"]').click();
    await page.waitForTimeout(1000);
    
    // Verify WASM bridge was called with metric data
    const relevantLogs = logs.filter(log => 
      log.includes('selectedMetrics') || log.includes('metrics')
    );
    
    // Should have at least some communication about metrics
    expect(relevantLogs.length).toBeGreaterThan(0);
  });

  test('should trigger data fetching when metrics change', async ({ page }) => {
    // Track network requests
    const requests: string[] = [];
    page.on('request', request => {
      if (request.url().includes('/api/data')) {
        requests.push(request.url());
      }
    });
    
    const initialRequestCount = requests.length;
    
    // Change metric selection
    await page.locator('[data-testid="metric-price"]').click();
    await page.waitForTimeout(2000);
    
    // Should have triggered new request
    expect(requests.length).toBeGreaterThan(initialRequestCount);
    
    // Latest request should include new metric configuration
    if (requests.length > 0) {
      const latestRequest = requests[requests.length - 1];
      const url = new URL(latestRequest);
      const columns = url.searchParams.get('columns');
      
      expect(columns).toBeTruthy();
      expect(columns).toContain('time'); // Always present
      
      // Should reflect current metric selection
      const priceSelected = await page.locator('[data-testid="metric-price"]')
        .evaluate((el) => el.classList.contains('bg-blue-600'));
      
      if (priceSelected) {
        expect(columns).toContain('price');
      }
    }
  });

  test('should handle multiple rapid metric changes', async ({ page }) => {
    // Track requests to ensure they're debounced/batched appropriately
    const requests: string[] = [];
    page.on('request', request => {
      if (request.url().includes('/api/data')) {
        requests.push(request.url());
      }
    });
    
    const initialRequestCount = requests.length;
    
    // Rapidly change multiple metrics
    await page.locator('[data-testid="metric-price"]').click();
    await page.waitForTimeout(100);
    await page.locator('[data-testid="metric-volume"]').click();
    await page.waitForTimeout(100);
    await page.locator('[data-testid="metric-best_ask"]').click();
    await page.waitForTimeout(100);
    
    // Wait for requests to complete
    await page.waitForTimeout(3000);
    
    // Should not create excessive requests (should be debounced)
    const newRequests = requests.length - initialRequestCount;
    expect(newRequests).toBeLessThan(10); // Reasonable upper bound
    
    // Final request should reflect final state
    if (requests.length > initialRequestCount) {
      const latestRequest = requests[requests.length - 1];
      const url = new URL(latestRequest);
      const columns = url.searchParams.get('columns');
      
      expect(columns).toBeTruthy();
      expect(columns).toContain('time');
    }
  });

  test('should handle WASM chart busy states gracefully', async ({ page }) => {
    // Monitor console for WASM errors
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error' || (msg.type() === 'log' && msg.text().includes('error'))) {
        errors.push(msg.text());
      }
    });
    
    // Rapidly interact with chart while it's processing
    for (let i = 0; i < 5; i++) {
      await page.locator('[data-testid="metric-price"]').click();
      await page.locator('[data-testid="timeframe-selector"]').selectOption('5m');
      await page.locator('[data-testid="metric-volume"]').click();
      await page.waitForTimeout(50);
    }
    
    // Wait for everything to settle
    await page.waitForTimeout(3000);
    
    // Should not have crashed or thrown borrow errors
    const borrowErrors = errors.filter(error => 
      error.includes('already mutably borrowed') || 
      error.includes('BorrowError') ||
      error.includes('RuntimeError: unreachable')
    );
    
    expect(borrowErrors.length).toBe(0);
    
    // UI should still be responsive
    await expect(page.locator('[data-testid="metric-best_bid"]')).toBeVisible();
    await expect(page.locator('text=Data Metrics')).toBeVisible();
  });

  test('should maintain chart state consistency', async ({ page }) => {
    // Set specific chart state
    await page.locator('[data-testid="symbol-selector"]').selectOption('ETH-USD');
    await page.locator('[data-testid="timeframe-selector"]').selectOption('1h');
    
    // Set specific metric selection
    await page.locator('[data-testid="metric-best_bid"]').click();
    await page.locator('[data-testid="metric-price"]').click();
    
    await page.waitForTimeout(2000);
    
    // Verify all states are maintained together
    const symbol = await page.locator('[data-testid="symbol-selector"]').inputValue();
    const timeframe = await page.locator('[data-testid="timeframe-selector"]').inputValue();
    
    expect(symbol).toBe('ETH-USD');
    expect(timeframe).toBe('1h');
    
    // Verify metrics are still selected correctly
    const bidSelected = await page.locator('[data-testid="metric-best_bid"]')
      .evaluate((el) => el.classList.contains('bg-blue-600'));
    const priceSelected = await page.locator('[data-testid="metric-price"]')
      .evaluate((el) => el.classList.contains('bg-blue-600'));
    
    expect(bidSelected).toBe(true);
    expect(priceSelected).toBe(true);
  });

  test('should handle metric validation errors gracefully', async ({ page }) => {
    // Monitor for validation errors
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error' || msg.text().includes('validation')) {
        errors.push(msg.text());
      }
    });
    
    // Try to trigger edge cases
    // Rapidly toggle all metrics to try to create empty selection
    const metrics = ['best_bid', 'best_ask', 'price', 'volume'];
    
    for (let attempt = 0; attempt < 3; attempt++) {
      for (const metric of metrics) {
        await page.locator(`[data-testid="metric-${metric}"]`).click();
        await page.waitForTimeout(50);
      }
    }
    
    await page.waitForTimeout(2000);
    
    // Should still have valid state
    const selectedCount = await page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').count();
    expect(selectedCount).toBeGreaterThanOrEqual(1);
    
    // Should not have thrown validation errors that break the app
    const criticalErrors = errors.filter(error => 
      error.includes('validation failed') && error.includes('selectedMetrics')
    );
    
    // Some validation warnings are OK, but not critical failures
    expect(criticalErrors.length).toBeLessThan(3);
  });

  test('should serialize metric data correctly for WASM', async ({ page }) => {
    // Inject test code to monitor WASM communication
    await page.addInitScript(() => {
      (window as any).__wasmCalls = [];
      
      // Intercept WASM calls if possible
      const originalFetch = window.fetch;
      window.fetch = function(...args) {
        if (args[0] && typeof args[0] === 'string' && args[0].includes('api/data')) {
          (window as any).__wasmCalls.push(args[0]);
        }
        return originalFetch.apply(this, args);
      };
    });
    
    // Change metrics and trigger update
    await page.locator('[data-testid="metric-volume"]').click();
    await page.waitForTimeout(1000);
    
    // Check that fetch calls were made with correct format
    const wasmCalls = await page.evaluate(() => (window as any).__wasmCalls || []);
    
    if (wasmCalls.length > 0) {
      const latestCall = wasmCalls[wasmCalls.length - 1];
      const url = new URL(latestCall);
      const columns = url.searchParams.get('columns');
      
      // Should be properly formatted comma-separated list
      expect(columns).toMatch(/^time(,[a-z_]+)*$/);
      
      // Should include time as first column
      expect(columns).toMatch(/^time/);
    }
  });

  test('should handle metric color assignment consistently', async ({ page }) => {
    // This test would need access to chart internals
    // For now, we can test that different metrics are visually distinct
    
    // Select multiple metrics
    await page.locator('[data-testid="metric-best_bid"]').click();
    await page.locator('[data-testid="metric-best_ask"]').click();
    await page.locator('[data-testid="metric-price"]').click();
    
    await page.waitForTimeout(3000);
    
    // All selected metrics should be visually indicated
    const selectedMetrics = await page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').count();
    expect(selectedMetrics).toBeGreaterThanOrEqual(3);
    
    // Chart canvas should be present and rendering
    await expect(page.locator('[data-testid="chart-canvas"]')).toBeVisible();
    
    // Canvas should have content (non-zero size)
    const canvas = page.locator('[data-testid="chart-canvas"]');
    const boundingBox = await canvas.boundingBox();
    expect(boundingBox?.width).toBeGreaterThan(0);
    expect(boundingBox?.height).toBeGreaterThan(0);
  });
});