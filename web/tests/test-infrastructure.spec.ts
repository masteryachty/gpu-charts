import { test, expect } from '@playwright/test';
import { GraphTestUtils } from './helpers/test-utils';

test.describe('Test Infrastructure', () => {
  let utils: GraphTestUtils;
  
  test.beforeEach(async ({ page }) => {
    utils = new GraphTestUtils(page);
  });

  test('test data server should be accessible', async ({ page }) => {
    // Test that the test server is running and accessible
    const isAvailable = await utils.dataHelper.isTestServerAvailable();
    expect(isAvailable).toBe(true);
    
    // Test symbols endpoint
    const response = await page.request.get('http://localhost:8080/api/symbols');
    expect(response.ok()).toBe(true);
    
    const data = await response.json();
    expect(data).toHaveProperty('symbols');
    expect(Array.isArray(data.symbols)).toBe(true);
    expect(data.symbols.length).toBeGreaterThan(0);
  });

  test('test data server should serve mock data', async ({ page }) => {
    const testServerUrl = 'http://localhost:8080';
    const params = new URLSearchParams({
      symbol: 'BTC-USD',
      start: '1745322750',
      end: '1745326350',
      columns: 'time,price,volume'
    });
    
    const response = await page.request.get(`${testServerUrl}/api/data?${params}`);
    expect(response.ok()).toBe(true);
    
    // Check headers
    const headers = response.headers();
    expect(headers['x-data-columns']).toBeDefined();
    expect(headers['x-data-records']).toBeDefined();
    
    // Check that we get binary data
    const buffer = await response.body();
    expect(buffer.length).toBeGreaterThan(0);
  });

  test('app should load with test environment setup', async ({ page }) => {
    // Setup test environment
    await utils.setupTestEnvironment();
    
    // Navigate to app
    await utils.navigateToApp();
    
    // Should see the app interface
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible();
    
    // Check for test mode flags
    const testMode = await page.evaluate(() => (window as any).__TEST_MODE__);
    expect(testMode).toBe(true);
  });

  test('software rendering fallback should work', async ({ page }) => {
    // Enable software rendering fallback
    await utils.enableSoftwareRendering();
    
    // Navigate to app
    await utils.navigateToApp();
    
    // Should still see the canvas even with software rendering
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible();
    
    // Check for software rendering flags
    const forceSoftwareRendering = await page.evaluate(() => (window as any).__FORCE_SOFTWARE_RENDERING__);
    expect(forceSoftwareRendering).toBe(true);
  });

  test('mock data routing should work', async ({ page }) => {
    // Setup mock data
    await utils.dataHelper.mockSymbols(['TEST-USD', 'MOCK-BTC']);
    await utils.dataHelper.mockDataResponse({
      symbol: 'TEST-USD',
      records: 100,
      columns: ['time', 'price']
    });
    
    // Navigate to app
    await page.goto('/app');
    
    // Test that API calls are intercepted
    const symbolsResponse = await page.evaluate(async () => {
      const response = await fetch('/api/symbols');
      return await response.json();
    });
    
    expect(symbolsResponse.symbols).toEqual(['TEST-USD', 'MOCK-BTC']);
  });

  test('error handling should work', async ({ page }) => {
    // Setup API error mocking
    await utils.dataHelper.mockApiError('symbols', 500, 'Test error');
    
    // Navigate to app
    await page.goto('/app');
    
    // Should handle the error gracefully
    const errorResponse = await page.evaluate(async () => {
      try {
        const response = await fetch('/api/symbols');
        return { status: response.status, ok: response.ok };
      } catch (error) {
        return { error: error.message };
      }
    });
    
    expect(errorResponse.status).toBe(500);
    expect(errorResponse.ok).toBe(false);
  });

  test('WebGPU detection should work in test environment', async ({ page }) => {
    await page.goto('/app');
    
    const webgpuInfo = await page.evaluate(() => {
      return {
        hasWebGPU: 'gpu' in navigator,
        testMode: (window as any).__TEST_MODE__,
        disableWebGPU: (window as any).__DISABLE_WEBGPU__,
        forceSoftwareRendering: (window as any).__FORCE_SOFTWARE_RENDERING__
      };
    });
    
    console.log('WebGPU info:', webgpuInfo);
    
    // In the configured test environment, WebGPU might be available but disabled
    expect(typeof webgpuInfo.hasWebGPU).toBe('boolean');
  });
});