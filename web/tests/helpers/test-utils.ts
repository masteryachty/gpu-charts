import { Page, expect } from '@playwright/test';

export class GraphTestUtils {
  constructor(private page: Page) {}

  /**
   * Wait for WASM module to load and initialize
   */
  async waitForWasmLoad(timeout = 15000) {
    // Wait for loading overlay to disappear
    await this.page.waitForSelector('[data-testid="loading-overlay"]', { state: 'detached', timeout });
    
    // Additional check for canvas being ready
    await this.page.waitForFunction(() => {
      const canvas = document.getElementById('wasm-chart-canvas') as HTMLCanvasElement;
      return canvas && canvas.width > 0 && canvas.height > 0;
    }, { timeout });
  }

  /**
   * Wait for WebGPU to initialize if available
   */
  async waitForWebGPU(timeout = 10000) {
    const hasWebGPU = await this.page.evaluate(() => 'gpu' in navigator);
    
    if (hasWebGPU) {
      await this.page.waitForFunction(async () => {
        try {
          const adapter = await navigator.gpu.requestAdapter();
          return adapter !== null;
        } catch {
          return false;
        }
      }, { timeout });
    }
    
    return hasWebGPU;
  }

  /**
   * Navigate to app with test parameters
   */
  async navigateToApp(topic = 'BTC-usd', start = 1745322750, end = 1745691150) {
    // Navigate to the React app route
    await this.page.goto(`/app`);
  }

  /**
   * Wait for chart to be fully loaded and rendered
   */
  async waitForChartRender(timeout = 10000) {
    // Wait for canvas to appear
    await expect(this.page.locator('#wasm-chart-canvas')).toBeVisible({ timeout });
    
    // Wait for WASM to load
    await this.waitForWasmLoad();
    
    // Give time for initial render
    await this.page.waitForTimeout(1000);
  }

  /**
   * Perform zoom interaction on the chart
   */
  async zoomChart(deltaY: number, x?: number, y?: number) {
    const canvas = this.page.locator('#wasm-chart-canvas');
    
    if (x !== undefined && y !== undefined) {
      await this.page.mouse.move(x, y);
    } else {
      await canvas.hover();
    }
    
    await this.page.mouse.wheel(0, deltaY);
    await this.page.waitForTimeout(100); // Allow zoom to process
  }

  /**
   * Perform pan interaction on the chart
   */
  async panChart(fromX: number, fromY: number, toX: number, toY: number) {
    await this.page.mouse.move(fromX, fromY);
    await this.page.mouse.down();
    await this.page.mouse.move(toX, toY);
    await this.page.mouse.up();
    await this.page.waitForTimeout(100); // Allow pan to process
  }

  /**
   * Get canvas dimensions
   */
  async getCanvasDimensions() {
    const canvas = this.page.locator('#wasm-chart-canvas');
    return await canvas.boundingBox();
  }

  /**
   * Check for memory leaks by monitoring heap size
   */
  async checkMemoryUsage() {
    return await this.page.evaluate(() => {
      const memory = (performance as any).memory;
      return memory ? {
        used: memory.usedJSHeapSize,
        total: memory.totalJSHeapSize,
        limit: memory.jsHeapSizeLimit
      } : null;
    });
  }

  /**
   * Simulate network conditions
   */
  async simulateSlowNetwork() {
    const client = await this.page.context().newCDPSession(this.page);
    await client.send('Network.emulateNetworkConditions', {
      offline: false,
      downloadThroughput: 50 * 1024, // 50kb/s
      uploadThroughput: 20 * 1024,   // 20kb/s
      latency: 500 // 500ms
    });
  }

  /**
   * Block all network requests
   */
  async blockNetwork() {
    await this.page.route('**/*', route => route.abort());
  }

  /**
   * Mock successful data response
   */
  async mockDataResponse(data: any) {
    await this.page.route('**/api/**', route => {
      route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify(data)
      });
    });
  }

  /**
   * Mock network error
   */
  async mockNetworkError() {
    await this.page.route('**/api/**', route => {
      route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal server error' })
      });
    });
  }

  /**
   * Take screenshot for debugging
   */
  async takeDebugScreenshot(name: string) {
    await this.page.screenshot({
      path: `test-results/debug-${name}-${Date.now()}.png`,
      fullPage: true
    });
  }

  /**
   * Get console errors
   */
  async getConsoleErrors() {
    const errors: string[] = [];
    
    this.page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });
    
    return errors;
  }

  /**
   * Check if WebGPU is supported and working
   */
  async isWebGPUWorking() {
    return await this.page.evaluate(async () => {
      if (!('gpu' in navigator)) return false;
      
      try {
        const adapter = await navigator.gpu.requestAdapter();
        if (!adapter) return false;
        
        const device = await adapter.requestDevice();
        return device !== null;
      } catch {
        return false;
      }
    });
  }

  // Static methods for backward compatibility
  static async waitForWasmLoad(page: Page, timeout = 15000) {
    const utils = new GraphTestUtils(page);
    return utils.waitForWasmLoad(timeout);
  }

  static async waitForWebGPU(page: Page, timeout = 10000) {
    const utils = new GraphTestUtils(page);
    return utils.waitForWebGPU(timeout);
  }

  static async waitForChartRender(page: Page, timeout = 10000) {
    const utils = new GraphTestUtils(page);
    return utils.waitForChartRender(timeout);
  }

  static async isWebGPUWorking(page: Page) {
    const utils = new GraphTestUtils(page);
    return utils.isWebGPUWorking();
  }

  static async measureMemoryUsage(page: Page) {
    const utils = new GraphTestUtils(page);
    return utils.checkMemoryUsage();
  }

  static async zoomChart(page: Page, deltaY: number, x?: number, y?: number) {
    const utils = new GraphTestUtils(page);
    return utils.zoomChart(deltaY, x, y);
  }

  static async panChart(page: Page, fromX: number, fromY: number, toX: number, toY: number) {
    const utils = new GraphTestUtils(page);
    return utils.panChart(fromX, fromY, toX, toY);
  }

  static async takeDebugScreenshot(page: Page, name: string) {
    const utils = new GraphTestUtils(page);
    return utils.takeDebugScreenshot(name);
  }

  static async navigateToApp(page: Page, topic = 'BTC-usd', start = 1745322750, end = 1745691150) {
    const utils = new GraphTestUtils(page);
    return utils.navigateToApp(topic, start, end);
  }
}

/**
 * Test data generators
 */
export const TestData = {
  generateTimeSeriesData: (points = 100, startTime = 1745322750) => {
    return Array.from({ length: points }, (_, i) => ({
      timestamp: startTime + i * 60, // 1 minute intervals
      value: Math.random() * 1000 + Math.sin(i * 0.1) * 100
    }));
  },

  validTimeRange: { start: 1745322750, end: 1745691150 },
  invalidTimeRange: { start: 9999999999, end: 9999999990 },
  
  topics: {
    BTC: 'BTC-usd',
    ETH: 'ETH-usd',
    SENSOR: 'sensor_data'
  }
};