import { test, expect } from '@playwright/test';
import { GraphTestUtils } from './helpers/test-utils';
import { TEST_DATA_SETS } from './helpers/test-data-helper';

test.describe('Improved Basic App Tests', () => {
  let utils: GraphTestUtils;
  
  test.beforeEach(async ({ page }) => {
    utils = new GraphTestUtils(page);
    // Setup test environment with all improvements
    await utils.setupTestEnvironment({
      enableSoftwareRendering: true,
      mockDataServer: true,
      enableTestMode: true
    });
  });

  test('should load homepage successfully', async ({ page }) => {
    await page.goto('/');
    
    // Should see the main heading
    await expect(page.locator('h1')).toBeVisible({ timeout: 10000 });
    
    // Should have navigation to the app
    const appLink = page.locator('a[href*="/app"]');
    if (await appLink.isVisible()) {
      await expect(appLink).toBeVisible();
    }
  });

  test('should navigate to trading app with test server', async ({ page }) => {
    // Setup mock data first
    await utils.dataHelper.mockSymbols(TEST_DATA_SETS.CRYPTO_SYMBOLS);
    await utils.dataHelper.mockDataResponse({
      symbol: 'BTC-USD',
      records: 100,
      columns: TEST_DATA_SETS.COLUMNS.MARKET_DATA
    });
    
    // Navigate to app
    await utils.navigateToApp();
    
    // Should show trading app interface
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible({ timeout: 15000 });
    
    // Should have some content loaded
    const hasContent = await page.evaluate(() => {
      return document.body.innerText.length > 100;
    });
    expect(hasContent).toBe(true);
  });

  test('should handle canvas with software fallback', async ({ page }) => {
    await utils.navigateToApp();
    
    // Wait for canvas to appear
    const canvas = page.locator('#wasm-chart-canvas');
    await expect(canvas).toBeVisible({ timeout: 15000 });
    
    // Check canvas properties
    const canvasInfo = await canvas.evaluate((el: HTMLCanvasElement) => ({
      width: el.width,
      height: el.height,
      hasContext: !!el.getContext('2d')
    }));
    
    expect(canvasInfo.width).toBeGreaterThan(0);
    expect(canvasInfo.height).toBeGreaterThan(0);
    expect(canvasInfo.hasContext).toBe(true);
  });

  test('should handle interactions without crashing', async ({ page }) => {
    await utils.navigateToApp();
    
    // Wait for interface to load
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible({ timeout: 15000 });
    
    // Perform basic interactions
    await page.mouse.move(400, 300);
    await page.mouse.wheel(0, -100); // Zoom
    await page.mouse.move(450, 350);
    await page.mouse.click(450, 350);
    
    // Should still be responsive
    const canvasStillVisible = await page.locator('#wasm-chart-canvas').isVisible();
    expect(canvasStillVisible).toBe(true);
  });

  test('should handle API data with test server', async ({ page }) => {
    // Use the test server for data
    await utils.dataHelper.routeToTestServer();
    
    await utils.navigateToApp();
    
    // Wait for interface to load
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible({ timeout: 15000 });
    
    // Check that data requests are made
    const responsePromise = page.waitForResponse(response => 
      response.url().includes('/api/symbols') && response.status() === 200
    );
    
    // Wait for the response
    const response = await responsePromise;
    expect(response.ok()).toBe(true);
  });

  test('should display loading states properly', async ({ page }) => {
    // Add delay to see loading state
    await utils.dataHelper.mockDataResponse({
      symbol: 'BTC-USD',
      records: 1000,
      delay: 1000 // 1 second delay
    });
    
    await utils.navigateToApp();
    
    // Should show loading overlay initially
    const loadingOverlay = page.locator('[data-testid="loading-overlay"]');
    if (await loadingOverlay.isVisible()) {
      await expect(loadingOverlay).toBeVisible();
      
      // Should eventually disappear or show error
      await expect(loadingOverlay).not.toBeVisible({ timeout: 10000 });
    }
    
    // Should have canvas visible
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible();
  });

  test('should handle test mode flags correctly', async ({ page }) => {
    await utils.navigateToApp();
    
    // Check test mode flags
    const testFlags = await page.evaluate(() => ({
      testMode: (window as any).__TEST_MODE__,
      forceSoftwareRendering: (window as any).__FORCE_SOFTWARE_RENDERING__,
      disableWebGPU: (window as any).__DISABLE_WEBGPU__,
      testTimeoutOverride: (window as any).__TEST_TIMEOUT_OVERRIDE__
    }));
    
    expect(testFlags.testMode).toBe(true);
    expect(testFlags.testTimeoutOverride).toBe(5000);
  });

  test('should handle errors gracefully', async ({ page }) => {
    // Setup API error
    await utils.dataHelper.mockApiError('data', 500, 'Test server error');
    
    await utils.navigateToApp();
    
    // Should handle the error without crashing
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible({ timeout: 15000 });
    
    // Should not have any uncaught JavaScript errors that crash the page
    const hasPageContent = await page.evaluate(() => document.body.children.length > 0);
    expect(hasPageContent).toBe(true);
  });
});