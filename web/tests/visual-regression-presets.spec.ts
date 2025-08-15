import { test, expect, Page } from '@playwright/test';

// Fixed test data for consistency
const TEST_PARAMS = {
  symbol: 'BTC-USD',
  start: 1745322750,
  end: 1745409150
};

const TEST_URL = `http://localhost:3003/app?topic=${TEST_PARAMS.symbol}&start=${TEST_PARAMS.start}&end=${TEST_PARAMS.end}`;

async function waitForChartToLoad(page: Page) {
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

async function getAvailablePresets(page: Page): Promise<string[]> {
  // Get all options from the preset selector
  const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
  await presetSelector.waitFor({ state: 'visible', timeout: 10000 });
  
  const options = await presetSelector.locator('option').allTextContents();
  // Filter out the placeholder option
  return options.filter(opt => opt !== 'Select a Preset' && opt !== '');
}

test.describe('Visual Regression - Chart Presets', () => {
  test.setTimeout(120000); // 2 minutes per test
  
  test.beforeEach(async ({ page }) => {
    // Set viewport
    await page.setViewportSize({ width: 1280, height: 720 });
    
    // Navigate to the app
    console.log('Navigating to:', TEST_URL);
    await page.goto(TEST_URL, { waitUntil: 'networkidle' });
    
    // Wait for chart to be ready
    await waitForChartToLoad(page);
  });

  test('Default chart view (no preset)', async ({ page }) => {
    // Take screenshot of default state
    await expect(page).toHaveScreenshot('default-chart-view.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'), // Mask timestamps
        page.locator('[data-testid="status-bar"]') // Mask status bar if present
      ],
      maxDiffPixels: 100
    });
  });

  test('All available presets', async ({ page }) => {
    const presets = await getAvailablePresets(page);
    console.log('Found presets:', presets);
    
    const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
    
    for (const preset of presets) {
      console.log(`Testing preset: ${preset}`);
      
      // Select the preset
      await presetSelector.selectOption(preset);
      
      // Wait for the preset to be applied
      await page.waitForTimeout(3000);
      
      // Take screenshot
      const screenshotName = `preset-${preset.toLowerCase().replace(/\s+/g, '-')}.png`;
      await expect(page).toHaveScreenshot(screenshotName, {
        fullPage: false,
        animations: 'disabled',
        mask: [
          page.locator('.text-xs.text-gray-500'),
          page.locator('[data-testid="status-bar"]')
        ],
        maxDiffPixels: 100
      });
      
      console.log(`Screenshot saved: ${screenshotName}`);
    }
  });

  test('Preset with metric toggles', async ({ page }) => {
    const presets = await getAvailablePresets(page);
    if (presets.length === 0) {
      console.log('No presets available, skipping test');
      return;
    }
    
    const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
    
    // Select first preset
    const firstPreset = presets[0];
    console.log(`Testing preset with toggles: ${firstPreset}`);
    await presetSelector.selectOption(firstPreset);
    await page.waitForTimeout(2000);
    
    // Check if there are metric toggles
    const metricCheckboxes = page.locator('input[type="checkbox"]');
    const checkboxCount = await metricCheckboxes.count();
    
    if (checkboxCount > 0) {
      console.log(`Found ${checkboxCount} metric toggles`);
      
      // Take screenshot with all metrics on
      await expect(page).toHaveScreenshot(`preset-${firstPreset.toLowerCase().replace(/\s+/g, '-')}-all-metrics.png`, {
        fullPage: false,
        animations: 'disabled',
        mask: [
          page.locator('.text-xs.text-gray-500'),
          page.locator('[data-testid="status-bar"]')
        ],
        maxDiffPixels: 100
      });
      
      // Toggle first metric off
      const firstCheckbox = metricCheckboxes.first();
      await firstCheckbox.uncheck();
      await page.waitForTimeout(2000);
      
      // Take screenshot with first metric off
      await expect(page).toHaveScreenshot(`preset-${firstPreset.toLowerCase().replace(/\s+/g, '-')}-metric-toggled.png`, {
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

  test('Chart interactions with preset', async ({ page }) => {
    const presets = await getAvailablePresets(page);
    if (presets.length === 0) {
      console.log('No presets available, using default view');
    } else {
      // Select first preset
      const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
      await presetSelector.selectOption(presets[0]);
      await page.waitForTimeout(2000);
    }
    
    const canvas = page.locator('canvas#webgpu-canvas');
    
    // Initial state
    await expect(page).toHaveScreenshot('interaction-initial.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
    
    // Zoom in
    await canvas.hover();
    await page.mouse.wheel(0, -200);
    await page.waitForTimeout(2000);
    
    await expect(page).toHaveScreenshot('interaction-zoomed.png', {
      fullPage: false,
      animations: 'disabled',
      mask: [
        page.locator('.text-xs.text-gray-500'),
        page.locator('[data-testid="status-bar"]')
      ],
      maxDiffPixels: 100
    });
    
    // Pan
    const box = await canvas.boundingBox();
    if (box) {
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + 150, box.y + box.height / 2);
      await page.mouse.up();
      await page.waitForTimeout(2000);
      
      await expect(page).toHaveScreenshot('interaction-panned.png', {
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

  test.skip('Different viewports with presets', async ({ page, context }) => {
    const viewports = [
      { name: 'desktop-4k', width: 2560, height: 1440 },
      { name: 'desktop-full-hd', width: 1920, height: 1080 },
      { name: 'laptop', width: 1366, height: 768 },
      { name: 'tablet-landscape', width: 1024, height: 768 },
      { name: 'tablet-portrait', width: 768, height: 1024 }
    ];
    
    const presets = await getAvailablePresets(page);
    const testPreset = presets.length > 0 ? presets[0] : null;
    
    for (const viewport of viewports) {
      console.log(`Testing viewport: ${viewport.name}`);
      
      // Set viewport and reload
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.reload({ waitUntil: 'networkidle' });
      await waitForChartToLoad(page);
      
      // Apply preset if available
      if (testPreset) {
        const presetSelector = page.locator('select:has(option:text("Select a Preset"))');
        await presetSelector.selectOption(testPreset);
        await page.waitForTimeout(2000);
      }
      
      // Take screenshot
      await expect(page).toHaveScreenshot(`viewport-${viewport.name}.png`, {
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