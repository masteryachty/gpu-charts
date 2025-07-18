import { test, expect } from '@playwright/test';

test.describe('Candlestick Chart Features', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to app
    await page.goto('/app');
    
    // Wait for React to load
    await page.waitForTimeout(2000);
    
    // Wait for canvas to be present
    await page.waitForSelector('#wasm-canvas', { timeout: 30000 });
    
    // Check if this is a mock environment (for CI/testing)
    const isMock = await page.evaluate(() => {
      const canvas = document.querySelector('#wasm-canvas');
      return canvas?.getAttribute('data-mock') === 'true';
    });
    
    if (isMock) {
      console.log('Running in mock mode');
    }
    
    // Additional wait for chart initialization
    await page.waitForTimeout(3000);
  });

  test('should display chart type controls', async ({ page }) => {
    // Check that chart controls are visible
    const chartControls = page.locator('[data-testid="chart-controls"]');
    await expect(chartControls).toBeVisible({ timeout: 10000 });
    
    // Check for chart type buttons
    const lineButton = page.locator('button:has-text("Line")');
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    
    await expect(lineButton).toBeVisible();
    await expect(candlestickButton).toBeVisible();
  });

  test('should switch to candlestick chart', async ({ page }) => {
    // Find buttons
    const lineButton = page.locator('button:has-text("Line")');
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    
    // Initially should show line chart (check for active class)
    await expect(lineButton).toHaveClass(/bg-blue-600/);
    await expect(candlestickButton).toHaveClass(/bg-gray-600/);
    
    // Click candlestick button
    await candlestickButton.click();
    await page.waitForTimeout(1000);
    
    // Button states should change
    await expect(candlestickButton).toHaveClass(/bg-blue-600/);
    await expect(lineButton).toHaveClass(/bg-gray-600/);
  });

  test('should show timeframe dropdown when candlestick is selected', async ({ page }) => {
    // Switch to candlestick
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    await page.waitForTimeout(500);
    
    // Timeframe dropdown should appear
    const timeframeLabel = page.locator('text=Timeframe:');
    const timeframeSelect = page.locator('select[data-testid="timeframe-select"]');
    
    await expect(timeframeLabel).toBeVisible();
    await expect(timeframeSelect).toBeVisible();
    
    // Check default value
    const selectedValue = await timeframeSelect.inputValue();
    expect(selectedValue).toBe('60');
  });

  test('should hide timeframe dropdown when switching back to line', async ({ page }) => {
    // Switch to candlestick first
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    await page.waitForTimeout(500);
    
    // Verify timeframe is visible
    const timeframeLabel = page.locator('text=Timeframe:');
    await expect(timeframeLabel).toBeVisible();
    
    // Switch back to line
    const lineButton = page.locator('button:has-text("Line")');
    await lineButton.click();
    await page.waitForTimeout(500);
    
    // Timeframe should be hidden
    await expect(timeframeLabel).not.toBeVisible();
  });

  test('should change candlestick timeframe', async ({ page }) => {
    // Switch to candlestick
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    await page.waitForTimeout(500);
    
    const timeframeSelect = page.locator('select[data-testid="timeframe-select"]');
    
    // Test each timeframe option
    const timeframes = [
      { value: '60', label: '1 minute' },
      { value: '300', label: '5 minutes' },
      { value: '900', label: '15 minutes' },
      { value: '3600', label: '1 hour' }
    ];
    
    for (const timeframe of timeframes) {
      await timeframeSelect.selectOption(timeframe.value);
      await page.waitForTimeout(500);
      
      const selectedValue = await timeframeSelect.inputValue();
      expect(selectedValue).toBe(timeframe.value);
    }
  });

  test('should maintain chart type when changing timeframe', async ({ page }) => {
    // Switch to candlestick
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    await page.waitForTimeout(500);
    
    // Change timeframe
    const timeframeSelect = page.locator('select[data-testid="timeframe-select"]');
    await timeframeSelect.selectOption('300');
    await page.waitForTimeout(500);
    
    // Should still be on candlestick
    await expect(candlestickButton).toHaveClass(/bg-blue-600/);
  });

  test('should render candlestick data', async ({ page }) => {
    // Switch to candlestick
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    
    // Wait for potential re-render
    await page.waitForTimeout(2000);
    
    // Canvas should still be present
    const canvas = page.locator('#wasm-canvas');
    await expect(canvas).toBeVisible();
    
    // Take screenshot for visual verification
    await canvas.screenshot({ path: 'test-results/candlestick-chart.png' });
  });

  test('should handle zoom and pan in candlestick mode', async ({ page }) => {
    // Switch to candlestick
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    await page.waitForTimeout(1000);
    
    const canvas = page.locator('#wasm-canvas');
    const canvasBox = await canvas.boundingBox();
    
    if (canvasBox) {
      // Test zoom
      await page.mouse.move(canvasBox.x + canvasBox.width / 2, canvasBox.y + canvasBox.height / 2);
      await page.mouse.wheel(0, -100); // Zoom in
      await page.waitForTimeout(500);
      
      // Test pan (simulate drag)
      const startX = canvasBox.x + canvasBox.width / 2;
      const startY = canvasBox.y + canvasBox.height / 2;
      await page.mouse.move(startX, startY);
      await page.mouse.down();
      await page.mouse.move(startX + 100, startY);
      await page.mouse.up();
      await page.waitForTimeout(500);
    }
    
    // Canvas should still be visible
    await expect(canvas).toBeVisible();
  });

  test('should persist timeframe when switching chart types', async ({ page }) => {
    // Set to candlestick with 5 minute timeframe
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    await page.waitForTimeout(500);
    
    const timeframeSelect = page.locator('select[data-testid="timeframe-select"]');
    await timeframeSelect.selectOption('300');
    await page.waitForTimeout(500);
    
    // Switch to line chart
    const lineButton = page.locator('button:has-text("Line")');
    await lineButton.click();
    await page.waitForTimeout(500);
    
    // Switch back to candlestick
    await candlestickButton.click();
    await page.waitForTimeout(500);
    
    // Timeframe should still be 5 minutes
    const selectedValue = await timeframeSelect.inputValue();
    expect(selectedValue).toBe('300');
  });

  test('should handle rapid chart type switching', async ({ page }) => {
    const lineButton = page.locator('button:has-text("Line")');
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    
    // Rapidly switch between chart types
    for (let i = 0; i < 5; i++) {
      await lineButton.click();
      await page.waitForTimeout(200);
      await candlestickButton.click();
      await page.waitForTimeout(200);
    }
    
    // Should end on candlestick
    await expect(candlestickButton).toHaveClass(/bg-blue-600/);
    
    // Canvas should still be functional
    const canvas = page.locator('#wasm-canvas');
    await expect(canvas).toBeVisible();
  });

  test('should log OHLC aggregation to console', async ({ page }) => {
    // Collect console logs
    const consoleLogs: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'log' || msg.type() === 'info') {
        consoleLogs.push(msg.text());
      }
    });
    
    // Switch to candlestick
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    await page.waitForTimeout(2000);
    
    // Check for OHLC-related logs
    const ohlcLogs = consoleLogs.filter(log => 
      log.includes('OHLC') || 
      log.includes('candle') || 
      log.includes('Candlestick') ||
      log.includes('chart type')
    );
    
    // Should have some OHLC-related activity
    expect(ohlcLogs.length).toBeGreaterThan(0);
    console.log('OHLC-related logs:', ohlcLogs);
  });
});

// Additional edge case tests
test.describe('Candlestick Edge Cases', () => {
  test('should handle missing data gracefully', async ({ page }) => {
    await page.goto('/app');
    await page.waitForTimeout(3000);
    
    // Switch to candlestick without waiting for data
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    
    // Should not crash
    const canvas = page.locator('#wasm-canvas');
    await expect(canvas).toBeVisible();
  });

  test('should handle invalid timeframe gracefully', async ({ page }) => {
    await page.goto('/app');
    await page.waitForTimeout(3000);
    
    // Switch to candlestick
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    await candlestickButton.click();
    await page.waitForTimeout(500);
    
    // Try to set invalid timeframe via JavaScript
    const result = await page.evaluate(() => {
      const select = document.querySelector('select[data-testid="timeframe-select"]') as HTMLSelectElement;
      if (select) {
        // This should be prevented by the select options
        select.value = '999999';
        const event = new Event('change', { bubbles: true });
        select.dispatchEvent(event);
        return select.value;
      }
      return null;
    });
    
    // Should either reject invalid value or handle gracefully
    const canvas = page.locator('#wasm-canvas');
    await expect(canvas).toBeVisible();
  });
});