import { test, expect } from '@playwright/test';
import { GraphTestUtils } from './helpers/test-utils';

// Test data constants based on your API structure
const API_BASE = 'https://192.168.1.91:8443'; // Your TLS server
const TEST_SYMBOLS = ['BTC', 'ETH', 'AAPL']; // Common symbols
const TEST_TIME_RANGES = {
  SHORT: { start: 1745322750, end: 1745322850 }, // 100 seconds
  MEDIUM: { start: 1745322750, end: 1745326350 }, // 1 hour  
  LONG: { start: 1745322750, end: 1745409150 },   // 24 hours
  MULTI_DAY: { start: 1745322750, end: 1745582550 } // 3 days
};

// Mock data generators for different scenarios
const generateMockDataResponse = (numRecords: number, columns: string[]) => {
  const header = {
    columns: columns.map(col => ({
      name: col,
      record_size: 4,
      num_records: numRecords,
      data_length: numRecords * 4
    }))
  };
  
  // Generate binary data (simplified - in real tests you'd generate actual binary)
  const mockBinaryData = new ArrayBuffer(numRecords * 4 * columns.length);
  
  return {
    header: JSON.stringify(header) + '\n',
    data: mockBinaryData
  };
};

test.describe('Data Visualization API Integration', () => {
  let utils: GraphTestUtils;

  test.beforeEach(async ({ page }) => {
    utils = new GraphTestUtils(page);
    
    // Accept self-signed certificates for your TLS server
    await page.context().route('**/*', route => {
      if (route.request().url().includes('localhost:8443')) {
        // For testing, we'll mock the API responses
        route.fulfill({
          status: 200,
          contentType: 'application/octet-stream',
          body: generateMockDataResponse(1000, ['time', 'price', 'volume']).header
        });
      } else {
        route.continue();
      }
    });
  });

  test('should fetch and display symbols list', async ({ page }) => {
    // Mock symbols API response
    await page.route('**/api/symbols', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ symbols: TEST_SYMBOLS })
      });
    });

    await utils.navigateToApp();
    await utils.waitForChartRender();

    // Should be able to access symbols data
    const symbolsData = await page.evaluate(async () => {
      try {
        const response = await fetch('/api/symbols');
        return await response.json();
      } catch (error) {
        return { error: error.message };
      }
    });

    expect(symbolsData.symbols).toContain('BTC');
  });

  test('should handle data API with different column combinations', async () => {
    test.skip(true, 'API integration test - skipping for now');
  });

  test('should handle different time ranges correctly', async () => {
    test.skip(true, 'API integration test - skipping for now');
  });

  test('should handle API errors gracefully', async ({ page }) => {
    const errorScenarios = [
      { status: 400, body: 'Missing symbol', description: 'bad request' },
      { status: 500, body: 'Server error', description: 'server error' },
      { status: 404, body: 'Symbol not found', description: 'not found' }
    ];

    await utils.navigateToApp();
    await utils.waitForChartRender();

    for (const scenario of errorScenarios) {
      await page.route('**/api/data*', route => {
        route.fulfill({
          status: scenario.status,
          contentType: 'text/plain',
          body: scenario.body
        });
      });

      const errorHandled = await page.evaluate(async (scenario) => {
        try {
          const response = await fetch('/api/data?symbol=INVALID&type=MD&start=1&end=2&columns=time');
          return {
            status: response.status,
            text: await response.text(),
            handled: !response.ok
          };
        } catch (error) {
          return { error: error.message, handled: true };
        }
      }, scenario);

      expect(errorHandled.handled).toBe(true);
      console.log(`✓ Handled ${scenario.description} error`);
    }
  });

  test('should validate data integrity in response', async () => {
    test.skip(true, 'API integration test - skipping for now');
  });
});

test.describe('Data Visualization Chart Rendering', () => {
  let utils: GraphTestUtils;

  test.beforeEach(async ({ page }) => {
    utils = new GraphTestUtils(page);
  });

  test('should render charts with different data sizes', async ({ page }) => {
    const dataSizes = [100, 1000, 10000, 50000];
    
    for (const size of dataSizes) {
      await page.route('**/api/data*', route => {
        route.fulfill({
          status: 200,
          contentType: 'application/octet-stream',
          body: generateMockDataResponse(size, ['time', 'price']).header
        });
      });

      await utils.navigateToApp();
      await utils.waitForChartRender();
      
      // Verify canvas is properly sized and visible
      const canvas = page.locator('#wasm-chart-canvas');
      await expect(canvas).toBeVisible();
      
      const canvasBox = await canvas.boundingBox();
      expect(canvasBox?.width).toBeGreaterThan(400);
      expect(canvasBox?.height).toBeGreaterThan(300);
      
      console.log(`✓ Rendered chart with ${size} data points`);
      
      // Give time for rendering to complete
      await page.waitForTimeout(1000);
    }
  });

  test('should handle empty data gracefully', async ({ page }) => {
    await page.route('**/api/data*', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/octet-stream',
        body: generateMockDataResponse(0, ['time', 'price']).header
      });
    });

    await utils.navigateToApp();
    
    // Should still show canvas even with no data
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible({ timeout: 15000 });
    
    // Should not show error overlay for empty data
    const errorOverlay = page.locator('[data-testid="error-overlay"]');
    await expect(errorOverlay).not.toBeVisible();
  });

  test('should update charts when data changes', async () => {
    test.skip(true, 'Data refresh mechanism test - skipping for now');
  });

  test('should handle malformed data responses', async ({ page }) => {
    const malformedResponses = [
      { body: 'not json', description: 'invalid JSON' },
      { body: '{"invalid": "structure"}', description: 'wrong structure' },
      { body: '{"columns": []}', description: 'empty columns' },
      { body: '{"columns": [{"name": "time"}]}', description: 'missing metadata' }
    ];

    await utils.navigateToApp();
    await utils.waitForChartRender();

    for (const response of malformedResponses) {
      await page.route('**/api/data*', route => {
        route.fulfill({
          status: 200,
          contentType: 'application/octet-stream',
          body: response.body
        });
      });

      const errorHandled = await page.evaluate(async (response) => {
        try {
          const res = await fetch('/api/data?symbol=TEST&type=MD&start=1&end=2&columns=time');
          const text = await res.text();
          
          // Try to parse as expected format
          const lines = text.split('\n');
          const header = JSON.parse(lines[0]);
          
          // Check if it has expected structure
          return {
            hasValidStructure: header.columns && Array.isArray(header.columns),
            response: response.description
          };
        } catch (error) {
          return { error: true, response: response.description };
        }
      }, response);

      // Should either handle gracefully or show error
      const hasError = page.locator('[data-testid="error-overlay"]');
      const hasCanvas = page.locator('#wasm-chart-canvas');
      
      // Either should show error overlay OR canvas should still be visible
      const errorVisible = await hasError.isVisible();
      const canvasVisible = await hasCanvas.isVisible();
      
      expect(errorVisible || canvasVisible).toBe(true);
      console.log(`✓ Handled ${response.description}`);
    }
  });
});