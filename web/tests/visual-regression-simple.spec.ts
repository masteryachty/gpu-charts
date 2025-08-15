import { test, expect } from '@playwright/test';

const TEST_URL = 'http://localhost:3000/app?topic=BTC-USD&start=1745322750&end=1745409150';

test.describe('Visual Regression Tests - Simple', () => {
  test.beforeEach(async ({ page }) => {
    // Set viewport
    await page.setViewportSize({ width: 1280, height: 720 });
  });

  test('Chart loads and displays correctly', async ({ page }) => {
    // Navigate to the app
    await page.goto(TEST_URL);
    
    // Wait for the canvas to be visible
    await page.waitForSelector('canvas#webgpu-canvas', { 
      state: 'visible',
      timeout: 30000 
    });
    
    // Wait for any loading overlay to disappear
    const loadingOverlay = page.locator('[data-testid="loading-overlay"]');
    if (await loadingOverlay.isVisible()) {
      await loadingOverlay.waitFor({ state: 'hidden', timeout: 30000 });
    }
    
    // Give the chart time to render
    await page.waitForTimeout(5000);
    
    // Take screenshot
    await expect(page).toHaveScreenshot('chart-default-view.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [page.locator('.text-xs.text-gray-500')] // Mask timestamp
    });
  });

  test('Preset selection works', async ({ page }) => {
    // Navigate to the app
    await page.goto(TEST_URL);
    
    // Wait for the canvas
    await page.waitForSelector('canvas#webgpu-canvas', { 
      state: 'visible',
      timeout: 30000 
    });
    
    // Wait for any loading to complete
    await page.waitForTimeout(5000);
    
    // Find and interact with preset selector
    const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
    if (await presetSelector.isVisible()) {
      // Get all available options
      const options = await presetSelector.locator('option').allTextContents();
      console.log('Available presets:', options);
      
      // Select first real preset (skip "Select a Preset")
      if (options.length > 1) {
        await presetSelector.selectOption({ index: 1 });
        await page.waitForTimeout(3000);
        
        // Take screenshot with preset applied
        await expect(page).toHaveScreenshot('chart-with-preset.png', {
          fullPage: false,
          animations: 'disabled',
          mask: [page.locator('.text-xs.text-gray-500')]
        });
      }
    }
  });

  test('Different viewport sizes', async ({ page }) => {
    const viewports = [
      { name: 'desktop', width: 1920, height: 1080 },
      { name: 'laptop', width: 1366, height: 768 },
      { name: 'tablet', width: 768, height: 1024 }
    ];
    
    for (const viewport of viewports) {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto(TEST_URL);
      
      // Wait for canvas
      await page.waitForSelector('canvas#webgpu-canvas', { 
        state: 'visible',
        timeout: 30000 
      });
      
      await page.waitForTimeout(5000);
      
      // Take screenshot for each viewport
      await expect(page).toHaveScreenshot(`chart-${viewport.name}.png`, {
        fullPage: false,
        animations: 'disabled',
        mask: [page.locator('.text-xs.text-gray-500')]
      });
    }
  });

  test('Chart interactions', async ({ page }) => {
    await page.goto(TEST_URL);
    
    // Wait for canvas
    const canvas = page.locator('canvas#webgpu-canvas');
    await canvas.waitFor({ state: 'visible', timeout: 30000 });
    await page.waitForTimeout(5000);
    
    // Take initial screenshot
    await expect(page).toHaveScreenshot('chart-before-interaction.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [page.locator('.text-xs.text-gray-500')]
    });
    
    // Perform zoom interaction
    await canvas.hover();
    await page.mouse.wheel(0, -100);
    await page.waitForTimeout(2000);
    
    // Take screenshot after zoom
    await expect(page).toHaveScreenshot('chart-after-zoom.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [page.locator('.text-xs.text-gray-500')]
    });
    
    // Pan interaction
    const box = await canvas.boundingBox();
    if (box) {
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + 100, box.y + box.height / 2);
      await page.mouse.up();
      await page.waitForTimeout(2000);
      
      // Take screenshot after pan
      await expect(page).toHaveScreenshot('chart-after-pan.png', {
        fullPage: false,
        animations: 'disabled',
        mask: [page.locator('.text-xs.text-gray-500')]
      });
    }
  });
});