import { test, expect } from '@playwright/test';

test.describe('Preset Toggle Functionality', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await page.goto('/app');
    
    // Wait for WASM to initialize
    await page.waitForFunction(() => (window as any).__WASM_CHART_READY__ === true, { timeout: 30000 });
    
    // Select Market Data preset
    const presetDropdown = page.locator('select').filter({ hasText: 'Select a Preset' });
    await presetDropdown.selectOption('Market Data');
    
    // Wait for checkboxes to appear
    await page.waitForSelector('label:has-text("Ask") input[type="checkbox"]', { timeout: 5000 });
  });

  test('should toggle Ask checkbox off and on again', async ({ page }) => {
    // Find Ask checkbox
    const askCheckbox = page.locator('label').filter({ hasText: 'Ask' }).locator('input[type="checkbox"]');
    
    // Verify it's initially checked
    await expect(askCheckbox).toBeChecked();
    
    // Listen for console logs to debug
    page.on('console', msg => {
      if (msg.text().includes('[PresetSection]')) {
        console.log('Browser console:', msg.text());
      }
    });
    
    // Uncheck the Ask checkbox
    console.log('Test: Unchecking Ask checkbox');
    await askCheckbox.uncheck();
    
    // Verify it's unchecked
    await expect(askCheckbox).not.toBeChecked();
    
    // Wait a moment for the chart to update
    await page.waitForTimeout(500);
    
    // Check the Ask checkbox again
    console.log('Test: Re-checking Ask checkbox');
    await askCheckbox.check();
    
    // Verify it's checked again
    await expect(askCheckbox).toBeChecked();
    
    // Wait for chart update
    await page.waitForTimeout(500);
    
    // The checkbox should remain checked
    await expect(askCheckbox).toBeChecked();
  });

  test('should toggle multiple checkboxes independently', async ({ page }) => {
    const bidCheckbox = page.locator('label').filter({ hasText: 'Bid' }).locator('input[type="checkbox"]');
    const askCheckbox = page.locator('label').filter({ hasText: 'Ask' }).locator('input[type="checkbox"]');
    const tradesCheckbox = page.locator('label').filter({ hasText: 'Trades' }).locator('input[type="checkbox"]');
    
    // All should be checked initially
    await expect(bidCheckbox).toBeChecked();
    await expect(askCheckbox).toBeChecked();
    await expect(tradesCheckbox).toBeChecked();
    
    // Uncheck Ask
    await askCheckbox.uncheck();
    await expect(askCheckbox).not.toBeChecked();
    await expect(bidCheckbox).toBeChecked(); // Others should remain checked
    await expect(tradesCheckbox).toBeChecked();
    
    // Uncheck Bid
    await bidCheckbox.uncheck();
    await expect(bidCheckbox).not.toBeChecked();
    await expect(askCheckbox).not.toBeChecked(); // Ask should remain unchecked
    await expect(tradesCheckbox).toBeChecked();
    
    // Re-check Ask
    await askCheckbox.check();
    await expect(askCheckbox).toBeChecked();
    await expect(bidCheckbox).not.toBeChecked(); // Bid should remain unchecked
    await expect(tradesCheckbox).toBeChecked();
    
    // Re-check Bid
    await bidCheckbox.check();
    await expect(bidCheckbox).toBeChecked();
    await expect(askCheckbox).toBeChecked();
    await expect(tradesCheckbox).toBeChecked();
  });
});