import { test, expect } from '@playwright/test';

test.describe('Preset Loading', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await page.goto('/app');
    
    // Wait for WASM to initialize
    await page.waitForFunction(() => (window as any).__WASM_CHART_READY__ === true, { timeout: 30000 });
  });

  test('should show preset options immediately when Market Data is selected', async ({ page }) => {
    // Click on the preset dropdown
    const presetDropdown = page.locator('select').filter({ hasText: 'Select a Preset' });
    await expect(presetDropdown).toBeVisible();
    
    // Select Market Data preset
    await presetDropdown.selectOption('Market Data');
    
    // Checkboxes should appear immediately without waiting for data
    const bidCheckbox = page.locator('label').filter({ hasText: 'Bid' }).locator('input[type="checkbox"]');
    const askCheckbox = page.locator('label').filter({ hasText: 'Ask' }).locator('input[type="checkbox"]');
    const tradesCheckbox = page.locator('label').filter({ hasText: 'Trades' }).locator('input[type="checkbox"]');
    const midCheckbox = page.locator('label').filter({ hasText: 'Mid' }).locator('input[type="checkbox"]');
    
    // All checkboxes should be visible immediately
    await expect(bidCheckbox).toBeVisible({ timeout: 1000 }); // 1 second timeout to ensure it's immediate
    await expect(askCheckbox).toBeVisible({ timeout: 1000 });
    await expect(tradesCheckbox).toBeVisible({ timeout: 1000 });
    await expect(midCheckbox).toBeVisible({ timeout: 1000 });
    
    // Verify they are checked by default
    await expect(bidCheckbox).toBeChecked();
    await expect(askCheckbox).toBeChecked();
    await expect(tradesCheckbox).toBeChecked();
    await expect(midCheckbox).toBeChecked();
    
    // Loading indicator should only appear next to the preset label
    const loadingIndicator = page.locator('text=Loading data...');
    if (await loadingIndicator.isVisible()) {
      // It should be next to the label, not blocking the checkboxes
      const presetLabel = page.locator('label').filter({ hasText: 'Preset' });
      await expect(loadingIndicator).toBeNear(presetLabel);
    }
  });

  test('should allow toggling checkboxes while data is loading', async ({ page }) => {
    // Select Market Data preset
    const presetDropdown = page.locator('select').filter({ hasText: 'Select a Preset' });
    await presetDropdown.selectOption('Market Data');
    
    // Find Ask checkbox
    const askCheckbox = page.locator('label').filter({ hasText: 'Ask' }).locator('input[type="checkbox"]');
    await expect(askCheckbox).toBeVisible({ timeout: 1000 });
    
    // Uncheck the Ask checkbox immediately
    await askCheckbox.uncheck();
    await expect(askCheckbox).not.toBeChecked();
    
    // The checkbox should remain interactive even if data is still loading
    await askCheckbox.check();
    await expect(askCheckbox).toBeChecked();
  });

  test('should clear checkboxes when preset is deselected', async ({ page }) => {
    // Select Market Data preset
    const presetDropdown = page.locator('select').filter({ hasText: 'Select a Preset' });
    await presetDropdown.selectOption('Market Data');
    
    // Wait for checkboxes to appear
    const bidCheckbox = page.locator('label').filter({ hasText: 'Bid' }).locator('input[type="checkbox"]');
    await expect(bidCheckbox).toBeVisible({ timeout: 1000 });
    
    // Clear the preset
    await presetDropdown.selectOption('');
    
    // Checkboxes should disappear
    await expect(bidCheckbox).not.toBeVisible();
  });
});