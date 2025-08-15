import { test, expect } from '@playwright/test';

// Fixed test data for consistency
const TEST_PARAMS = {
  symbol: 'BTC-USD',
  start: 1745322750,
  end: 1745409150
};

const TEST_URL = `http://localhost:3003/app?topic=${TEST_PARAMS.symbol}&start=${TEST_PARAMS.start}&end=${TEST_PARAMS.end}`;

async function waitForChartToLoad(page) {
  console.log('Waiting for chart to load...');
  
  // Wait for the canvas element
  await page.waitForSelector('canvas#webgpu-canvas', { 
    state: 'visible',
    timeout: 30000 
  });
  
  // Wait for loading overlay to disappear
  await page.waitForSelector('[data-testid="loading-overlay"]', { 
    state: 'hidden',
    timeout: 60000 
  }).catch(() => {
    console.log('Loading overlay not found or already hidden');
  });
  
  // Wait for canvas to be initialized
  await page.waitForFunction(
    () => {
      const canvas = document.querySelector('canvas#webgpu-canvas');
      if (!canvas) return false;
      const initialized = canvas.getAttribute('data-initialized') === 'true';
      return initialized;
    },
    { timeout: 60000 }
  );
  
  // Extra wait for rendering to complete
  await page.waitForTimeout(3000);
  console.log('Chart loaded successfully');
}

test.describe('Visual Regression - Metric Toggles', () => {
  test.setTimeout(120000);
  
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto(TEST_URL, { waitUntil: 'networkidle' });
    await waitForChartToLoad(page);
  });

  test('Market Data preset - all metrics visible', async ({ page }) => {
    const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
    await presetSelector.selectOption('Market Data');
    await page.waitForTimeout(2000);
    
    // Ensure all checkboxes are checked
    const checkboxes = page.locator('input[type="checkbox"]');
    const count = await checkboxes.count();
    
    for (let i = 0; i < count; i++) {
      const checkbox = checkboxes.nth(i);
      const isChecked = await checkbox.isChecked();
      if (!isChecked) {
        await checkbox.check();
      }
    }
    
    await page.waitForTimeout(2000);
    
    await expect(page).toHaveScreenshot('market-data-all-metrics.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
  });

  test('Market Data preset - bid/ask only', async ({ page }) => {
    const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
    await presetSelector.selectOption('Market Data');
    await page.waitForTimeout(2000);
    
    // Get all checkboxes
    const checkboxes = page.locator('input[type="checkbox"]');
    const count = await checkboxes.count();
    
    // Uncheck all except bid/ask related
    for (let i = 0; i < count; i++) {
      const checkbox = checkboxes.nth(i);
      const label = await checkbox.locator('xpath=../..').textContent();
      
      if (label && !label.toLowerCase().includes('bid') && !label.toLowerCase().includes('ask')) {
        await checkbox.uncheck();
      }
    }
    
    await page.waitForTimeout(2000);
    
    await expect(page).toHaveScreenshot('market-data-bid-ask-only.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
  });

  test('Market Data preset - price only', async ({ page }) => {
    const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
    await presetSelector.selectOption('Market Data');
    await page.waitForTimeout(2000);
    
    // Uncheck all checkboxes
    const checkboxes = page.locator('input[type="checkbox"]');
    const count = await checkboxes.count();
    
    for (let i = 0; i < count; i++) {
      await checkboxes.nth(i).uncheck();
    }
    
    await page.waitForTimeout(2000);
    
    await expect(page).toHaveScreenshot('market-data-price-only.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
  });

  test('Candlestick preset - default view', async ({ page }) => {
    const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
    await presetSelector.selectOption('Candlestick');
    await page.waitForTimeout(3000);
    
    await expect(page).toHaveScreenshot('candlestick-default.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
  });

  test('Candlestick preset - with metric toggles', async ({ page }) => {
    const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
    await presetSelector.selectOption('Candlestick');
    await page.waitForTimeout(2000);
    
    // Check if there are any toggles for candlestick
    const checkboxes = page.locator('input[type="checkbox"]');
    const count = await checkboxes.count();
    
    if (count > 0) {
      // Toggle first checkbox off
      await checkboxes.first().uncheck();
      await page.waitForTimeout(2000);
      
      await expect(page).toHaveScreenshot('candlestick-toggled.png', {
        fullPage: false,
        animations: 'disabled',
        mask: [
          page.locator('.text-xs.text-gray-500'),
          page.locator('[data-testid="status-bar"]')
        ],
        maxDiffPixels: 100
      });
    }
  });
});