import { Page, expect } from '@playwright/test';
import { Chart } from '../../pkg/wasm_bridge.js';

/**
 * Visual regression testing utilities for WebGPU charts
 */

// Standard viewport sizes for consistent testing
export const VIEWPORT_SIZES = {
  desktop: { width: 1920, height: 1080 },
  laptop: { width: 1366, height: 768 },
  tablet: { width: 768, height: 1024 },
  mobile: { width: 375, height: 667 }
} as const;

// Fixed timestamps for consistent data
export const TEST_DATA_RANGES = {
  // Using specific timestamps for BTC-USD data
  day: {
    symbol: 'BTC-USD',
    start: 1745322750,  // Recent data range
    end: 1745409150,
    description: '1 day of BTC-USD data'
  },
  week: {
    symbol: 'BTC-USD', 
    start: 1745086350,
    end: 1745691150,
    description: '1 week of BTC-USD data'
  },
  hour: {
    symbol: 'ETH-USD',
    start: 1745688000,
    end: 1745691600,
    description: '1 hour of ETH-USD data'
  }
} as const;

export interface VisualTestConfig {
  viewport?: keyof typeof VIEWPORT_SIZES;
  dataRange?: keyof typeof TEST_DATA_RANGES;
  preset?: string;
  waitForStable?: boolean;
  timeout?: number;
}

/**
 * Wait for WebGPU to be available and initialized
 */
export async function waitForWebGPU(page: Page, timeout = 30000): Promise<boolean> {
  return await page.waitForFunction(
    () => {
      // Check if WebGPU is available
      if (!('gpu' in navigator)) {
        console.warn('WebGPU not available in navigator');
        return false;
      }
      return true;
    },
    { timeout }
  ).then(() => true).catch(() => false);
}

/**
 * Wait for WASM module to be loaded and chart instance to be ready
 */
export async function waitForChartReady(page: Page, timeout = 30000): Promise<void> {
  // Wait for canvas element
  await page.waitForSelector('canvas#webgpu-canvas', { timeout });
  
  // Wait for chart instance to be available in the component
  await page.waitForFunction(
    () => {
      const canvas = document.querySelector('canvas#webgpu-canvas');
      if (!canvas) return false;
      
      // Check if canvas itself is initialized
      const initialized = canvas.getAttribute('data-initialized') === 'true';
      
      // Check if React has rendered and chart is initialized
      // The chart instance is passed through React props/state
      const chartContainer = canvas.closest('[data-chart-ready="true"]');
      
      return initialized && chartContainer !== null;
    },
    { timeout }
  );

  // Additional wait for WebGPU initialization
  await page.waitForTimeout(2000);
}

/**
 * Wait for data to be fully loaded and rendered
 */
export async function waitForDataLoaded(page: Page, timeout = 30000): Promise<void> {
  // Wait for data fetch to complete
  await page.waitForFunction(
    () => {
      // Check for loading indicators to disappear
      const loadingElements = document.querySelectorAll('[data-loading="true"]');
      if (loadingElements.length > 0) return false;
      
      // Check if status bar shows data loaded
      const statusBar = document.querySelector('[data-testid="status-bar"]');
      if (statusBar?.textContent?.includes('Loading')) return false;
      
      return true;
    },
    { timeout }
  );
  
  // Wait for render to stabilize
  await page.waitForTimeout(1000);
}

/**
 * Apply a preset and wait for it to be fully applied
 */
export async function applyPreset(page: Page, presetName: string): Promise<void> {
  // Select preset from dropdown
  const presetSelector = 'select:has(option:text("Select a Preset"))';
  await page.waitForSelector(presetSelector);
  await page.selectOption(presetSelector, presetName);
  
  // Wait for preset to be applied
  await page.waitForFunction(
    (preset) => {
      const presetIndicator = document.querySelector('.bg-blue-600.text-white.text-xs.rounded');
      return presetIndicator?.textContent?.includes(preset);
    },
    presetName,
    { timeout: 10000 }
  );
  
  // Wait for re-render
  await page.waitForTimeout(2000);
}

/**
 * Set up the page for visual testing
 */
export async function setupVisualTest(
  page: Page,
  config: VisualTestConfig = {}
): Promise<void> {
  const {
    viewport = 'desktop',
    dataRange = 'day',
    preset,
    waitForStable = true
  } = config;

  // Set viewport
  await page.setViewportSize(VIEWPORT_SIZES[viewport]);
  
  // Navigate to app with test data
  const range = TEST_DATA_RANGES[dataRange];
  const url = `/app?topic=${range.symbol}&start=${range.start}&end=${range.end}`;
  await page.goto(url);
  
  // Wait for WebGPU
  const hasWebGPU = await waitForWebGPU(page);
  if (!hasWebGPU) {
    throw new Error('WebGPU not available - cannot run visual tests');
  }
  
  // Wait for chart initialization
  await waitForChartReady(page);
  
  // Apply preset if specified
  if (preset) {
    await applyPreset(page, preset);
  }
  
  // Wait for data to be loaded
  await waitForDataLoaded(page);
  
  // Additional stabilization wait
  if (waitForStable) {
    await page.waitForTimeout(2000);
  }
}

/**
 * Take a screenshot with proper configuration for visual regression
 */
export async function takeChartScreenshot(
  page: Page,
  name: string,
  options: {
    fullPage?: boolean;
    clip?: { x: number; y: number; width: number; height: number };
    maskSelectors?: string[];
  } = {}
): Promise<Buffer> {
  const { fullPage = false, clip, maskSelectors = [] } = options;
  
  // Mask dynamic elements that might change
  const defaultMasks = [
    '[data-testid="status-bar"]', // Status bar with timestamps
    '.text-gray-400:has-text("Real-time")', // Real-time text
  ];
  
  const masks = [...defaultMasks, ...maskSelectors].map(selector => 
    page.locator(selector)
  );
  
  return await page.screenshot({
    fullPage,
    clip,
    mask: masks,
    animations: 'disabled',
    caret: 'hide'
  });
}

/**
 * Compare screenshot with baseline
 */
export async function compareWithBaseline(
  page: Page,
  testName: string,
  options: Parameters<typeof takeChartScreenshot>[2] = {}
): Promise<void> {
  const screenshot = await takeChartScreenshot(page, testName, options);
  
  // Use Playwright's built-in screenshot comparison
  expect(screenshot).toMatchSnapshot(`${testName}.png`, {
    maxDiffPixels: 100, // Allow up to 100 pixels difference
    threshold: 0.2, // Pixel diff threshold (0-1)
  });
}

/**
 * Get all available presets from the chart
 */
export async function getAvailablePresets(page: Page): Promise<string[]> {
  return await page.evaluate(() => {
    const options = Array.from(
      document.querySelectorAll('select:has(option:text("Select a Preset")) option')
    );
    return options
      .map(opt => opt.textContent || '')
      .filter(text => text && text !== 'Select a Preset');
  });
}

/**
 * Toggle a metric visibility
 */
export async function toggleMetric(page: Page, metricLabel: string): Promise<void> {
  const checkbox = page.locator(`label:has-text("${metricLabel}") input[type="checkbox"]`);
  await checkbox.click();
  await page.waitForTimeout(1000); // Wait for re-render
}

/**
 * Get visible metrics for current preset
 */
export async function getVisibleMetrics(page: Page): Promise<string[]> {
  return await page.evaluate(() => {
    const checkboxes = Array.from(
      document.querySelectorAll('label input[type="checkbox"]:checked')
    );
    return checkboxes.map(cb => {
      const label = cb.closest('label');
      return label?.textContent?.trim() || '';
    }).filter(Boolean);
  });
}

/**
 * Wait for canvas to be stable (no changes for specified duration)
 */
export async function waitForCanvasStable(
  page: Page,
  duration = 1000,
  maxWait = 10000
): Promise<void> {
  const startTime = Date.now();
  let lastImageData: string | null = null;
  let stableTime = 0;
  
  while (Date.now() - startTime < maxWait) {
    const currentImageData = await page.evaluate(() => {
      const canvas = document.querySelector('canvas#webgpu-canvas') as HTMLCanvasElement;
      if (!canvas) return null;
      
      const ctx = canvas.getContext('2d');
      if (!ctx) return null;
      
      // Get a sample of the canvas data (center region)
      const sampleSize = 100;
      const x = Math.floor((canvas.width - sampleSize) / 2);
      const y = Math.floor((canvas.height - sampleSize) / 2);
      
      try {
        const imageData = ctx.getImageData(x, y, sampleSize, sampleSize);
        // Convert to simple hash for comparison
        return Array.from(imageData.data.slice(0, 100)).join(',');
      } catch (e) {
        // May fail with WebGPU canvas
        return null;
      }
    });
    
    if (currentImageData === lastImageData) {
      stableTime += 200;
      if (stableTime >= duration) {
        break;
      }
    } else {
      stableTime = 0;
      lastImageData = currentImageData;
    }
    
    await page.waitForTimeout(200);
  }
}

/**
 * Clean up test artifacts
 */
export async function cleanupVisualTest(page: Page): Promise<void> {
  // Reset any test state
  await page.evaluate(() => {
    // Clear any test flags or state
    window.localStorage.clear();
    window.sessionStorage.clear();
  });
}