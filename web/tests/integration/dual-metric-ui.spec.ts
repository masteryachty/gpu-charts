import { test, expect } from '@playwright/test';

test.describe('Dual-Metric UI Integration', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await page.goto('/app?topic=BTC-USD&start=1745322750&end=1745691150');
    
    // Wait for the chart to initialize
    await page.waitForSelector('[data-testid="chart-canvas"]', { timeout: 10000 });
    
    // Wait for any initial data loading
    await page.waitForTimeout(2000);
  });

  test('should display metric selection controls', async ({ page }) => {
    // Check that metric controls are visible
    await expect(page.locator('text=Data Metrics')).toBeVisible();
    
    // Check that metric buttons exist
    await expect(page.locator('[data-testid="metric-best_bid"]')).toBeVisible();
    await expect(page.locator('[data-testid="metric-best_ask"]')).toBeVisible();
    await expect(page.locator('[data-testid="metric-price"]')).toBeVisible();
    await expect(page.locator('[data-testid="metric-volume"]')).toBeVisible();
    
    // Check metric count display
    await expect(page.locator('text=/Data Metrics \\(\\d+\\)/')).toBeVisible();
  });

  test('should handle metric selection changes', async ({ page }) => {
    // Initially should have default metrics selected
    const initialSelectedCount = await page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').count();
    expect(initialSelectedCount).toBeGreaterThan(0);
    
    // Add a new metric
    const priceButton = page.locator('[data-testid="metric-price"]');
    const wasSelected = await priceButton.evaluate((el) => el.classList.contains('bg-blue-600'));
    
    await priceButton.click();
    
    // Wait for state update
    await page.waitForTimeout(500);
    
    // Verify the selection state changed
    const isNowSelected = await priceButton.evaluate((el) => el.classList.contains('bg-blue-600'));
    expect(isNowSelected).toBe(!wasSelected);
    
    // Check that metric count updated
    const newCount = await page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').count();
    if (wasSelected) {
      expect(newCount).toBe(initialSelectedCount - 1);
    } else {
      expect(newCount).toBe(initialSelectedCount + 1);
    }
  });

  test('should prevent removing all metrics', async ({ page }) => {
    // First, select only one metric
    const metrics = ['best_bid', 'best_ask', 'price', 'volume'];
    
    // Deselect all but one
    for (let i = 0; i < metrics.length - 1; i++) {
      const button = page.locator(`[data-testid="metric-${metrics[i]}"]`);
      const isSelected = await button.evaluate((el) => el.classList.contains('bg-blue-600'));
      
      if (isSelected) {
        await button.click();
        await page.waitForTimeout(200);
      }
    }
    
    // Verify at least one metric is still selected
    const selectedCount = await page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').count();
    expect(selectedCount).toBeGreaterThanOrEqual(1);
    
    // Try to click the last selected metric
    const lastSelected = page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').first();
    const isDisabled = await lastSelected.evaluate((el) => 
      el.hasAttribute('disabled') || el.classList.contains('opacity-50')
    );
    
    if (!isDisabled) {
      await lastSelected.click();
      await page.waitForTimeout(200);
      
      // Should still have at least one selected
      const finalCount = await page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').count();
      expect(finalCount).toBeGreaterThanOrEqual(1);
    }
  });

  test('should trigger network requests with selected metrics', async ({ page }) => {
    // Listen for network requests
    const requests: string[] = [];
    page.on('request', (request) => {
      if (request.url().includes('/api/data')) {
        requests.push(request.url());
      }
    });
    
    // Change metric selection
    await page.locator('[data-testid="metric-volume"]').click();
    
    // Wait for potential network request
    await page.waitForTimeout(2000);
    
    // Check if any requests were made
    const latestRequest = requests[requests.length - 1];
    if (latestRequest) {
      // Verify the request includes columns parameter
      expect(latestRequest).toMatch(/columns=/);
      
      // Check that the columns parameter reflects the metric selection
      const url = new URL(latestRequest);
      const columns = url.searchParams.get('columns');
      expect(columns).toBeTruthy();
      expect(columns).toContain('time'); // Should always include time
    }
  });

  test('should show metric count correctly', async ({ page }) => {
    // Get initial metric count from UI
    const countText = await page.locator('text=/Data Metrics \\((\\d+)\\)/').textContent();
    const initialCount = parseInt(countText!.match(/\((\d+)\)/)![1]);
    
    // Add a metric if not selected
    const volumeButton = page.locator('[data-testid="metric-volume"]');
    const wasSelected = await volumeButton.evaluate((el) => el.classList.contains('bg-blue-600'));
    
    if (!wasSelected) {
      await volumeButton.click();
      await page.waitForTimeout(500);
      
      // Check that count increased
      const newCountText = await page.locator('text=/Data Metrics \\((\\d+)\\)/').textContent();
      const newCount = parseInt(newCountText!.match(/\((\d+)\)/)![1]);
      expect(newCount).toBe(initialCount + 1);
    }
  });

  test('should maintain metric selection across interactions', async ({ page }) => {
    // Select specific metrics
    const metricsToSelect = ['best_bid', 'price'];
    
    // First deselect all
    const allMetrics = ['best_bid', 'best_ask', 'price', 'volume'];
    for (const metric of allMetrics) {
      const button = page.locator(`[data-testid="metric-${metric}"]`);
      const isSelected = await button.evaluate((el) => el.classList.contains('bg-blue-600'));
      
      if (isSelected && !metricsToSelect.includes(metric)) {
        await button.click();
        await page.waitForTimeout(200);
      }
    }
    
    // Then select desired ones
    for (const metric of metricsToSelect) {
      const button = page.locator(`[data-testid="metric-${metric}"]`);
      const isSelected = await button.evaluate((el) => el.classList.contains('bg-blue-600'));
      
      if (!isSelected) {
        await button.click();
        await page.waitForTimeout(200);
      }
    }
    
    // Perform other chart interactions
    await page.locator('[data-testid="timeframe-selector"]').selectOption('5m');
    await page.waitForTimeout(1000);
    
    // Verify metric selection is preserved
    for (const metric of metricsToSelect) {
      const button = page.locator(`[data-testid="metric-${metric}"]`);
      const isSelected = await button.evaluate((el) => el.classList.contains('bg-blue-600'));
      expect(isSelected).toBe(true);
    }
  });

  test('should handle rapid metric selection changes', async ({ page }) => {
    // Rapidly toggle metrics
    const metrics = ['best_bid', 'best_ask', 'price', 'volume'];
    
    for (let i = 0; i < 3; i++) {
      for (const metric of metrics) {
        await page.locator(`[data-testid="metric-${metric}"]`).click();
        await page.waitForTimeout(100); // Short delay
      }
    }
    
    // Wait for all updates to settle
    await page.waitForTimeout(2000);
    
    // Verify UI is still responsive
    await expect(page.locator('text=Data Metrics')).toBeVisible();
    
    // Verify at least one metric is selected
    const selectedCount = await page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').count();
    expect(selectedCount).toBeGreaterThanOrEqual(1);
  });
});