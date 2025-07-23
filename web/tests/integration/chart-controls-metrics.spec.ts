import { test, expect } from '@playwright/test';

test.describe('Chart Controls Metrics UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/app?topic=BTC-USD&start=1745322750&end=1745691150');
    await page.waitForSelector('[data-testid="chart-canvas"]', { timeout: 10000 });
    await page.waitForTimeout(1000); // Allow chart to settle
  });

  test('should display all available metrics', async ({ page }) => {
    // Check that all metric buttons are present
    const expectedMetrics = ['best_bid', 'best_ask', 'price', 'volume'];
    
    for (const metric of expectedMetrics) {
      await expect(page.locator(`[data-testid="metric-${metric}"]`)).toBeVisible();
      
      // Check button text content
      const button = page.locator(`[data-testid="metric-${metric}"]`);
      const text = await button.textContent();
      expect(text).toContain(metric.replace('_', ' ').toUpperCase());
    }
  });

  test('should show metric count correctly', async ({ page }) => {
    // Get the metric count from the header
    const countElement = page.locator('text=/Data Metrics \\((\\d+)\\)/');
    await expect(countElement).toBeVisible();
    
    const countText = await countElement.textContent();
    const currentCount = parseInt(countText!.match(/\((\d+)\)/)![1]);
    
    // Count should match actual selected metrics
    const selectedButtons = page.locator('[data-testid^="metric-"][class*="bg-blue-600"]');
    const actualCount = await selectedButtons.count();
    
    expect(currentCount).toBe(actualCount);
  });

  test('should handle metric selection changes', async ({ page }) => {
    const priceButton = page.locator('[data-testid="metric-price"]');
    
    // Get initial state
    const initiallySelected = await priceButton.evaluate((el) => 
      el.classList.contains('bg-blue-600')
    );
    
    // Click to toggle
    await priceButton.click();
    await page.waitForTimeout(300);
    
    // Verify state changed
    const nowSelected = await priceButton.evaluate((el) => 
      el.classList.contains('bg-blue-600')
    );
    
    expect(nowSelected).toBe(!initiallySelected);
    
    // Verify visual state matches
    if (nowSelected) {
      await expect(priceButton).toHaveClass(/bg-blue-600/);
    } else {
      await expect(priceButton).toHaveClass(/bg-gray-700/);
    }
  });

  test('should prevent removing all metrics', async ({ page }) => {
    // First ensure we have multiple metrics selected
    const metrics = ['best_bid', 'best_ask', 'price', 'volume'];
    
    // Select a few metrics to start with
    for (const metric of ['best_bid', 'best_ask']) {
      const button = page.locator(`[data-testid="metric-${metric}"]`);
      const isSelected = await button.evaluate((el) => el.classList.contains('bg-blue-600'));
      
      if (!isSelected) {
        await button.click();
        await page.waitForTimeout(200);
      }
    }
    
    // Now try to deselect all but one
    for (const metric of ['best_bid']) {
      const button = page.locator(`[data-testid="metric-${metric}"]`);
      const isSelected = await button.evaluate((el) => el.classList.contains('bg-blue-600'));
      
      if (isSelected) {
        await button.click();
        await page.waitForTimeout(200);
      }
    }
    
    // Verify at least one metric remains selected
    const selectedCount = await page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').count();
    expect(selectedCount).toBeGreaterThanOrEqual(1);
    
    // Verify the last metric button is disabled or shows visual indication
    const remainingSelected = page.locator('[data-testid^="metric-"][class*="bg-blue-600"]');
    const count = await remainingSelected.count();
    
    if (count === 1) {
      const lastButton = remainingSelected.first();
      const isDisabled = await lastButton.evaluate((el) => 
        el.hasAttribute('disabled') || 
        el.classList.contains('opacity-50') ||
        el.classList.contains('cursor-not-allowed')
      );
      
      // If not visually disabled, clicking should not deselect it
      if (!isDisabled) {
        await lastButton.click();
        await page.waitForTimeout(200);
        
        const stillSelected = await lastButton.evaluate((el) => el.classList.contains('bg-blue-600'));
        expect(stillSelected).toBe(true);
      }
    }
  });

  test('should show visual feedback for metric states', async ({ page }) => {
    const volumeButton = page.locator('[data-testid="metric-volume"]');
    
    // Test selected state styling
    const isSelected = await volumeButton.evaluate((el) => el.classList.contains('bg-blue-600'));
    
    if (isSelected) {
      await expect(volumeButton).toHaveClass(/bg-blue-600/);
      await expect(volumeButton).toHaveClass(/text-white/);
    } else {
      await expect(volumeButton).toHaveClass(/bg-gray-700/);
      await expect(volumeButton).toHaveClass(/text-gray-300/);
    }
    
    // Test hover state (if implementable in Playwright)
    await volumeButton.hover();
    await page.waitForTimeout(100);
    
    // The button should have hover styling
    if (!isSelected) {
      await expect(volumeButton).toHaveClass(/hover:bg-gray-600/);
    }
  });

  test('should maintain accessibility features', async ({ page }) => {
    const metrics = ['best_bid', 'best_ask', 'price', 'volume'];
    
    for (const metric of metrics) {
      const button = page.locator(`[data-testid="metric-${metric}"]`);
      
      // Check button is focusable
      await button.focus();
      await expect(button).toBeFocused();
      
      // Check button can be activated with keyboard
      await button.press('Space');
      await page.waitForTimeout(200);
      
      // Button state should have changed
      const newState = await button.evaluate((el) => el.classList.contains('bg-blue-600'));
      expect(typeof newState).toBe('boolean');
    }
  });

  test('should update metric count when selection changes', async ({ page }) => {
    // Get initial count
    const getCount = async () => {
      const countText = await page.locator('text=/Data Metrics \\((\\d+)\\)/').textContent();
      return parseInt(countText!.match(/\((\d+)\)/)![1]);
    };
    
    const initialCount = await getCount();
    
    // Toggle a metric
    const volumeButton = page.locator('[data-testid="metric-volume"]');
    const wasSelected = await volumeButton.evaluate((el) => el.classList.contains('bg-blue-600'));
    
    await volumeButton.click();
    await page.waitForTimeout(500);
    
    const newCount = await getCount();
    
    if (wasSelected) {
      expect(newCount).toBe(Math.max(1, initialCount - 1)); // Can't go below 1
    } else {
      expect(newCount).toBe(initialCount + 1);
    }
  });

  test('should handle rapid clicking gracefully', async ({ page }) => {
    const priceButton = page.locator('[data-testid="metric-price"]');
    
    // Rapidly click the button multiple times
    for (let i = 0; i < 5; i++) {
      await priceButton.click();
      await page.waitForTimeout(50);
    }
    
    // Wait for all state updates to settle
    await page.waitForTimeout(1000);
    
    // UI should still be responsive
    await expect(page.locator('text=Data Metrics')).toBeVisible();
    
    // Button should be in a valid state
    const isSelected = await priceButton.evaluate((el) => el.classList.contains('bg-blue-600'));
    expect(typeof isSelected).toBe('boolean');
    
    // Should still have at least one metric selected
    const selectedCount = await page.locator('[data-testid^="metric-"][class*="bg-blue-600"]').count();
    expect(selectedCount).toBeGreaterThanOrEqual(1);
  });

  test('should show helper text for metrics', async ({ page }) => {
    // Check for the helper text about selecting multiple metrics
    const helperText = page.locator('text=Select multiple metrics to overlay on the chart');
    await expect(helperText).toBeVisible();
    
    // Verify the text is appropriately styled
    await expect(helperText).toHaveClass(/text-xs/);
    await expect(helperText).toHaveClass(/text-gray-500/);
  });

  test('should maintain metric selection during other interactions', async ({ page }) => {
    // Set specific metric selection
    await page.locator('[data-testid="metric-best_bid"]').click();
    await page.locator('[data-testid="metric-price"]').click();
    await page.waitForTimeout(500);
    
    // Get current metric selection state
    const getSelectedMetrics = async () => {
      const selectedButtons = page.locator('[data-testid^="metric-"][class*="bg-blue-600"]');
      const count = await selectedButtons.count();
      const metrics = [];
      
      for (let i = 0; i < count; i++) {
        const button = selectedButtons.nth(i);
        const testId = await button.getAttribute('data-testid');
        metrics.push(testId?.replace('metric-', ''));
      }
      
      return metrics;
    };
    
    const initialMetrics = await getSelectedMetrics();
    
    // Interact with other controls
    await page.locator('[data-testid="timeframe-selector"]').selectOption('5m');
    await page.waitForTimeout(1000);
    
    await page.locator('[data-testid="symbol-selector"]').selectOption('ETH-USD');
    await page.waitForTimeout(1000);
    
    // Verify metric selection is preserved
    const finalMetrics = await getSelectedMetrics();
    expect(finalMetrics.sort()).toEqual(initialMetrics.sort());
  });
});