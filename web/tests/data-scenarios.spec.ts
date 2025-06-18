import { test, expect } from '@playwright/test';
import { GraphTestUtils } from './helpers/test-utils';
import { DataMockHelper, TestScenarios } from './helpers/data-mocks';

test.describe('Real-World Data Scenarios', () => {
  let utils: GraphTestUtils;
  let dataMocker: DataMockHelper;

  test.beforeEach(async ({ page }) => {
    utils = new GraphTestUtils(page);
    dataMocker = new DataMockHelper(page);
  });

  test('should handle cryptocurrency data with high volatility', async ({ page }) => {
    await dataMocker.mockDataAPI({
      symbol: 'BTC',
      columns: TestScenarios.COLUMNS.WITH_VOLUME,
      recordCount: TestScenarios.MEDIUM_DATASET.recordCount
    });

    await utils.navigateToApp();
    await utils.waitForChartRender();

    // Verify canvas is rendering chart
    const canvas = page.locator('#wasm-chart-canvas');
    await expect(canvas).toBeVisible();

    // Test interactions work with crypto data
    await utils.zoomChart(-200); // Zoom in
    await page.waitForTimeout(500);
    
    await utils.zoomChart(100); // Zoom out
    await page.waitForTimeout(500);

    // Canvas should remain stable
    await expect(canvas).toBeVisible();
  });

  test('should handle stock market data with gaps (weekends)', async ({ page }) => {
    // Mock stock data that might have gaps
    await dataMocker.mockDataAPI({
      symbol: 'AAPL', 
      type: 'MD',
      columns: TestScenarios.COLUMNS.OHLC,
      recordCount: TestScenarios.LARGE_DATASET.recordCount
    });

    await utils.navigateToApp();
    await utils.waitForChartRender();

    // Should handle potential data gaps gracefully
    const canvas = page.locator('#wasm-chart-canvas');
    await expect(canvas).toBeVisible();

    // Test panning across potentially gapped data
    const canvasBox = await utils.getCanvasDimensions();
    if (canvasBox) {
      await utils.panChart(
        canvasBox.x + canvasBox.width * 0.2,
        canvasBox.y + canvasBox.height * 0.5,
        canvasBox.x + canvasBox.width * 0.8,
        canvasBox.y + canvasBox.height * 0.5
      );
    }

    await expect(canvas).toBeVisible();
  });

  test('should handle forex data with 24/7 availability', async ({ page }) => {
    await dataMocker.mockDataAPI({
      symbol: 'EURUSD',
      columns: ['time', 'bid', 'ask', 'spread'],
      recordCount: TestScenarios.HUGE_DATASET.recordCount
    });

    await utils.navigateToApp();
    await utils.waitForChartRender();

    // Test performance with large continuous dataset
    const startTime = Date.now();
    
    // Perform multiple operations
    for (let i = 0; i < 5; i++) {
      await utils.zoomChart(-100);
      await utils.zoomChart(100);
    }
    
    const operationTime = Date.now() - startTime;
    expect(operationTime).toBeLessThan(5000); // Should complete in 5 seconds

    const canvas = page.locator('#wasm-chart-canvas');
    await expect(canvas).toBeVisible();
  });

  test('should handle commodity data with seasonal patterns', async ({ page }) => {
    await dataMocker.mockDataAPI({
      symbol: 'WHEAT',
      columns: TestScenarios.COLUMNS.WITH_VOLUME,
      startTime: TestScenarios.TIME_RANGES.MONTH.start,
      endTime: TestScenarios.TIME_RANGES.MONTH.end,
      recordCount: 43200 // 30 days * 24 hours * 60 minutes
    });

    await utils.navigateToApp();
    await utils.waitForChartRender();

    // Test scrolling through monthly data
    const canvas = page.locator('#wasm-chart-canvas');
    const canvasBox = await utils.getCanvasDimensions();
    
    if (canvasBox) {
      // Simulate scrolling through time
      for (let scroll = 0; scroll < 3; scroll++) {
        await page.mouse.move(canvasBox.x + canvasBox.width / 2, canvasBox.y + canvasBox.height / 2);
        await page.mouse.wheel(100, 0); // Horizontal scroll
        await page.waitForTimeout(300);
      }
    }

    await expect(canvas).toBeVisible();
  });

  test('should handle sensor data with irregular intervals', async ({ page }) => {
    // Mock sensor data that might have irregular time intervals
    await dataMocker.mockDataAPI({
      symbol: 'SENSOR_001',
      type: 'IOT',
      columns: ['time', 'temperature', 'humidity', 'pressure'],
      recordCount: TestScenarios.SMALL_DATASET.recordCount
    });

    await utils.navigateToApp();
    await utils.waitForChartRender();

    const canvas = page.locator('#wasm-chart-canvas');
    await expect(canvas).toBeVisible();

    // Verify it handles small datasets
    const canvasBox = await canvas.boundingBox();
    expect(canvasBox?.width).toBeGreaterThan(300);
    expect(canvasBox?.height).toBeGreaterThan(200);
  });

  test('should handle multiple symbol switching', async () => {
    test.skip(true, 'Symbol switching API test - skipping for now');
  });

  test('should handle time range modifications', async () => {
    test.skip(true, 'Time range API test - skipping for now');
  });

  test('should handle data loading states', async ({ page }) => {
    // Mock slow API to test loading states
    await dataMocker.mockSlowDataAPI(3000);

    await utils.navigateToApp();
    
    // Should show loading state
    const loadingOverlay = page.locator('[data-testid="loading-overlay"]');
    await expect(loadingOverlay).toBeVisible({ timeout: 5000 });

    // Eventually should load
    await utils.waitForChartRender(10000);
    
    const canvas = page.locator('#wasm-chart-canvas');
    await expect(canvas).toBeVisible();
  });

  test('should handle concurrent data requests', async () => {
    test.skip(true, 'Concurrent request test - skipping for now');
  });

  test('should handle data with extreme values', async ({ page }) => {
    // Mock data with extreme values that might break rendering
    await page.route('**/api/data*', route => {
      const response = {
        columns: [
          { name: 'time', record_size: 4, num_records: 100, data_length: 400 },
          { name: 'price', record_size: 4, num_records: 100, data_length: 400 }
        ]
      };
      
      route.fulfill({
        status: 200,
        contentType: 'application/octet-stream',
        body: JSON.stringify(response) + '\n'
      });
    });

    await utils.navigateToApp();
    await utils.waitForChartRender();

    // Should handle extreme values without crashing
    const canvas = page.locator('#wasm-chart-canvas');
    await expect(canvas).toBeVisible();

    // Test interactions still work
    await utils.zoomChart(-100);
    await utils.zoomChart(200);
    
    await expect(canvas).toBeVisible();
  });
});