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
      console.log('Canvas initialized:', initialized);
      return initialized;
    },
    { timeout: 60000 }
  );
  
  // Extra wait for rendering to complete
  await page.waitForTimeout(3000);
  console.log('Chart loaded successfully');
}

test.describe('Visual Regression - Viewports', () => {
  test.setTimeout(120000);

  test('Desktop 4K viewport', async ({ page }) => {
    await page.setViewportSize({ width: 2560, height: 1440 });
    await page.goto(TEST_URL, { waitUntil: 'networkidle' });
    await waitForChartToLoad(page);
    
    await expect(page).toHaveScreenshot('viewport-desktop-4k.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
  });

  test('Desktop Full HD viewport', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.goto(TEST_URL, { waitUntil: 'networkidle' });
    await waitForChartToLoad(page);
    
    await expect(page).toHaveScreenshot('viewport-desktop-full-hd.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
  });

  test('Laptop viewport', async ({ page }) => {
    await page.setViewportSize({ width: 1366, height: 768 });
    await page.goto(TEST_URL, { waitUntil: 'networkidle' });
    await waitForChartToLoad(page);
    
    await expect(page).toHaveScreenshot('viewport-laptop.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
  });

  test('Tablet landscape viewport', async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 768 });
    await page.goto(TEST_URL, { waitUntil: 'networkidle' });
    await waitForChartToLoad(page);
    
    await expect(page).toHaveScreenshot('viewport-tablet-landscape.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
  });

  test('Tablet portrait viewport', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 1024 });
    await page.goto(TEST_URL, { waitUntil: 'networkidle' });
    await waitForChartToLoad(page);
    
    await expect(page).toHaveScreenshot('viewport-tablet-portrait.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
  });
});