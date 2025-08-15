import { test, expect } from '@playwright/test';
import {
  setupVisualTest,
  compareWithBaseline,
  getAvailablePresets,
  toggleMetric,
  getVisibleMetrics,
  waitForCanvasStable,
  cleanupVisualTest,
  VIEWPORT_SIZES,
  TEST_DATA_RANGES,
  applyPreset,
  waitForDataLoaded
} from './helpers/visual-test-utils';

// Only run in Chromium with WebGPU support
test.use({
  // Force Chromium for WebGPU
  browserName: 'chromium',
  // Disable animations
  launchOptions: {
    args: [
      '--enable-unsafe-webgpu',
      '--enable-webgpu-developer-features',
      '--use-angle=swiftshader',
    ]
  }
});

test.describe('Visual Regression Tests - Chart Presets', () => {
  test.beforeEach(async ({ page }) => {
    // Set up consistent test environment
    await page.addInitScript(() => {
      // Mock date to ensure consistent timestamps
      Date.now = () => 1745691150000;
      
      // Disable animations
      const style = document.createElement('style');
      style.innerHTML = `
        *, *::before, *::after {
          animation-duration: 0s !important;
          animation-delay: 0s !important;
          transition-duration: 0s !important;
          transition-delay: 0s !important;
        }
      `;
      document.head.appendChild(style);
    });
  });

  test.afterEach(async ({ page }) => {
    await cleanupVisualTest(page);
  });

  test('Market Data preset - default view', async ({ page }) => {
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day',
      preset: 'Market Data'
    });

    await waitForCanvasStable(page);
    await compareWithBaseline(page, 'market-data-default-desktop');
  });

  test('Market Data preset - all metrics visible', async ({ page }) => {
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day',
      preset: 'Market Data'
    });

    // Get all available metrics
    const metrics = await getVisibleMetrics(page);
    console.log('Available metrics:', metrics);

    await waitForCanvasStable(page);
    await compareWithBaseline(page, 'market-data-all-metrics-desktop');
  });

  test('Market Data preset - bid/ask only', async ({ page }) => {
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day',
      preset: 'Market Data'
    });

    // Toggle off other metrics, keep only bid/ask
    const metrics = await getVisibleMetrics(page);
    for (const metric of metrics) {
      if (!metric.includes('Bid') && !metric.includes('Ask')) {
        await toggleMetric(page, metric);
      }
    }

    await waitForCanvasStable(page);
    await compareWithBaseline(page, 'market-data-bid-ask-only-desktop');
  });

  test('All available presets - desktop view', async ({ page }) => {
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day'
    });

    const presets = await getAvailablePresets(page);
    console.log('Testing presets:', presets);

    for (const preset of presets) {
      await applyPreset(page, preset);
      await waitForDataLoaded(page);
      await waitForCanvasStable(page);
      
      const screenshotName = `preset-${preset.toLowerCase().replace(/\s+/g, '-')}-desktop`;
      await compareWithBaseline(page, screenshotName);
    }
  });

  test('Market Data preset - different time ranges', async ({ page }) => {
    const timeRanges: Array<keyof typeof TEST_DATA_RANGES> = ['hour', 'day', 'week'];

    for (const range of timeRanges) {
      await setupVisualTest(page, {
        viewport: 'desktop',
        dataRange: range,
        preset: 'Market Data'
      });

      await waitForCanvasStable(page);
      await compareWithBaseline(page, `market-data-range-${range}-desktop`);
    }
  });

  test('Market Data preset - responsive viewports', async ({ page }) => {
    const viewports: Array<keyof typeof VIEWPORT_SIZES> = ['desktop', 'laptop', 'tablet'];

    for (const viewport of viewports) {
      await setupVisualTest(page, {
        viewport,
        dataRange: 'day',
        preset: 'Market Data'
      });

      await waitForCanvasStable(page);
      await compareWithBaseline(page, `market-data-${viewport}`);
    }
  });

  test('Chart interactions - zoom and pan', async ({ page }) => {
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day',
      preset: 'Market Data'
    });

    const canvas = page.locator('canvas#webgpu-canvas');
    const box = await canvas.boundingBox();
    
    if (!box) {
      throw new Error('Canvas not found');
    }

    // Zoom in with mouse wheel
    await canvas.hover({ position: { x: box.width / 2, y: box.height / 2 } });
    await page.mouse.wheel(0, -100); // Zoom in
    await waitForCanvasStable(page);
    await compareWithBaseline(page, 'market-data-zoomed-in');

    // Pan right
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 4, box.y + box.height / 2);
    await page.mouse.up();
    await waitForCanvasStable(page);
    await compareWithBaseline(page, 'market-data-panned-right');

    // Reset view (zoom out)
    await page.mouse.wheel(0, 200); // Zoom out
    await waitForCanvasStable(page);
    await compareWithBaseline(page, 'market-data-reset-view');
  });

  test('Different symbols - visual consistency', async ({ page }) => {
    const symbols = ['BTC-USD', 'ETH-USD', 'ADA-USD'];

    for (const symbol of symbols) {
      // Navigate with specific symbol
      const range = TEST_DATA_RANGES.day;
      await page.goto(`/app?topic=${symbol}&start=${range.start}&end=${range.end}`);
      
      await setupVisualTest(page, {
        viewport: 'desktop',
        preset: 'Market Data'
      });

      await waitForCanvasStable(page);
      await compareWithBaseline(page, `market-data-${symbol.toLowerCase()}`);
    }
  });

  test('Quality settings comparison', async ({ page }) => {
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day',
      preset: 'Market Data'
    });

    // Note: If quality settings are available in UI, test them
    // For now, we'll just test the default quality
    await waitForCanvasStable(page);
    await compareWithBaseline(page, 'market-data-default-quality');
  });

  test('Edge cases - empty data handling', async ({ page }) => {
    // Test with invalid time range (future dates)
    const futureTime = Math.floor(Date.now() / 1000) + 86400 * 30; // 30 days in future
    await page.goto(`/app?topic=BTC-USD&start=${futureTime}&end=${futureTime + 3600}`);
    
    try {
      await setupVisualTest(page, {
        viewport: 'desktop',
        preset: 'Market Data',
        waitForStable: false
      });
    } catch (e) {
      // May timeout waiting for data
    }

    await page.waitForTimeout(3000);
    await compareWithBaseline(page, 'market-data-no-data');
  });

  test('Chart controls - UI state', async ({ page }) => {
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day',
      preset: 'Market Data'
    });

    // Focus on controls area
    const controlsArea = page.locator('.bg-gray-800.border.border-gray-600.rounded-lg');
    await controlsArea.scrollIntoViewIfNeeded();
    
    // Take screenshot of controls
    const controlsBox = await controlsArea.boundingBox();
    if (controlsBox) {
      await compareWithBaseline(page, 'controls-panel-market-data', {
        clip: controlsBox
      });
    }

    // Open preset dropdown
    const presetDropdown = page.locator('select:has(option:text("Select a Preset"))');
    await presetDropdown.click();
    await page.waitForTimeout(500);
    
    if (controlsBox) {
      await compareWithBaseline(page, 'controls-panel-dropdown-open', {
        clip: controlsBox
      });
    }
  });
});

test.describe('Visual Regression Tests - Performance', () => {
  test('Large dataset rendering', async ({ page }) => {
    // Test with week range for more data points
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'week',
      preset: 'Market Data'
    });

    await waitForCanvasStable(page, 2000); // Longer wait for large dataset
    await compareWithBaseline(page, 'market-data-large-dataset');
  });

  test('Rapid preset switching', async ({ page }) => {
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day'
    });

    const presets = await getAvailablePresets(page);
    
    // Rapidly switch between presets
    for (let i = 0; i < 3; i++) {
      for (const preset of presets.slice(0, 2)) { // Test first 2 presets
        await applyPreset(page, preset);
        await page.waitForTimeout(500); // Short wait
      }
    }

    // Final state after rapid switching
    await waitForCanvasStable(page);
    await compareWithBaseline(page, 'after-rapid-preset-switching');
  });
});

test.describe('Visual Regression Tests - Accessibility', () => {
  test('High contrast mode simulation', async ({ page }) => {
    await page.emulateMedia({ colorScheme: 'dark', forcedColors: 'active' });
    
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day',
      preset: 'Market Data'
    });

    await waitForCanvasStable(page);
    await compareWithBaseline(page, 'market-data-high-contrast');
  });

  test('Focus states and keyboard navigation', async ({ page }) => {
    await setupVisualTest(page, {
      viewport: 'desktop',
      dataRange: 'day',
      preset: 'Market Data'
    });

    // Tab through controls
    await page.keyboard.press('Tab');
    await page.waitForTimeout(500);
    await compareWithBaseline(page, 'controls-focus-state-1');

    await page.keyboard.press('Tab');
    await page.waitForTimeout(500);
    await compareWithBaseline(page, 'controls-focus-state-2');
  });
});