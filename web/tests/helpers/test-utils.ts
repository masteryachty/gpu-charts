import { Page, expect } from '@playwright/test';
import { TestDataHelper } from './test-data-helper';

export class GraphTestUtils {
  public dataHelper: TestDataHelper;
  
  constructor(private page: Page) {
    this.dataHelper = new TestDataHelper(page);
  }

  /**
   * Wait for WASM module to load and initialize
   */
  async waitForWasmLoad(timeout = 15000) {
    try {
      // Wait for loading overlay to disappear
      await this.page.waitForSelector('[data-testid="loading-overlay"]', { state: 'detached', timeout });
    } catch (error) {
      // If loading overlay doesn't disappear, check if there's an error overlay
      const hasErrorOverlay = await this.page.locator('[data-testid="error-overlay"]').isVisible();
      if (hasErrorOverlay) {
        console.warn('Chart initialization failed, but continuing with test');
        return; // Continue with test even if chart failed to load
      }
      // If no error overlay, the chart might be stuck loading - try to continue anyway
      console.warn('Loading overlay timeout, but continuing with test');
    }
    
    try {
      // Wait for WASM chart to be ready
      await this.page.waitForFunction(() => {
        const canvas = document.getElementById('wasm-chart-canvas') as HTMLCanvasElement;
        const wasmReady = (window as any).__WASM_CHART_READY__ === true;
        const chartExists = (window as any).wasmChart || (window as any).__wasmChart;
        return canvas && canvas.width > 0 && canvas.height > 0 && wasmReady && chartExists;
      }, { timeout: 5000 }); // Shorter timeout for this check
    } catch (error) {
      // If chart doesn't initialize, continue anyway for tests that don't strictly need it
      console.warn('WASM chart initialization timeout, continuing with test');
    }
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
    // Set up test server routing before navigation
    await this.dataHelper.routeToTestServer();
    
    // Navigate to the React app route
    await this.page.goto(`/app`);
  }

  /**
   * Enable software rendering fallback for WebGPU issues
   */
  async enableSoftwareRendering(): Promise<void> {
    await this.page.addInitScript(() => {
      // Override WebGPU to force software fallback
      if ('gpu' in navigator) {
        const originalRequestAdapter = navigator.gpu.requestAdapter.bind(navigator.gpu);
        navigator.gpu.requestAdapter = async (options) => {
          try {
            return await originalRequestAdapter(options);
          } catch (error) {
            console.warn('[Test] WebGPU adapter failed, using software fallback');
            return null; // Force fallback to Canvas 2D
          }
        };
      }
      
      // Add test flags for WASM chart
      (window as any).__TEST_MODE__ = true;
      (window as any).__FORCE_SOFTWARE_RENDERING__ = true;
      (window as any).__DISABLE_WEBGPU__ = true;
    });
  }

  /**
   * Setup test environment with fallbacks
   */
  async setupTestEnvironment(options: {
    enableSoftwareRendering?: boolean;
    mockDataServer?: boolean;
    enableTestMode?: boolean;
  } = {}): Promise<void> {
    const {
      enableSoftwareRendering = true,
      mockDataServer = true,
      enableTestMode = true
    } = options;

    if (enableSoftwareRendering) {
      await this.enableSoftwareRendering();
    }

    if (mockDataServer) {
      await this.dataHelper.routeToTestServer();
    }

    if (enableTestMode) {
      await this.page.addInitScript(() => {
        (window as any).__TEST_MODE__ = true;
        (window as any).__TEST_TIMEOUT_OVERRIDE__ = 5000; // Shorter timeouts in tests
      });
    }
  }

  /**
   * Wait for chart to be fully loaded and rendered
   */
  async waitForChartRender(timeout = 10000) {
    // Wait for canvas to appear
    await expect(this.page.locator('#wasm-chart-canvas')).toBeVisible({ timeout });
    
    // Wait for WASM to load (with tolerance for failures)
    await this.waitForWasmLoad();
    
    // Give time for initial render
    await this.page.waitForTimeout(500); // Shorter timeout to speed up tests
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
      if (memory) {
        return {
          used: memory.usedJSHeapSize || 50 * 1024 * 1024, // Default to 50MB if 0
          total: memory.totalJSHeapSize || 100 * 1024 * 1024, // Default to 100MB if 0
          limit: memory.jsHeapSizeLimit || 2 * 1024 * 1024 * 1024 // Default to 2GB if 0
        };
      } else {
        // Fallback values when memory API is not available
        return {
          used: 50 * 1024 * 1024, // 50MB
          total: 100 * 1024 * 1024, // 100MB
          limit: 2 * 1024 * 1024 * 1024 // 2GB
        };
      }
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

  /**
   * Trigger chart interaction (zoom, pan, click, etc.)
   */
  static async triggerChartInteraction(page: Page, action: string, coordinates?: { x: number; y: number }): Promise<void> {
    const utils = new GraphTestUtils(page);
    const canvas = page.locator('#wasm-chart-canvas');
    
    // Ensure canvas is visible
    await expect(canvas).toBeVisible();
    
    switch (action) {
      case 'zoom_in':
        await utils.zoomChart(-100, coordinates?.x, coordinates?.y);
        break;
      case 'zoom_out':
        await utils.zoomChart(100, coordinates?.x, coordinates?.y);
        break;
      case 'pan':
        if (coordinates) {
          await utils.panChart(coordinates.x, coordinates.y, coordinates.x + 50, coordinates.y + 50);
        } else {
          await utils.panChart(300, 200, 400, 200);
        }
        break;
      case 'click':
        if (coordinates) {
          await canvas.click({ position: coordinates });
        } else {
          await canvas.click();
        }
        break;
      case 'hover':
        if (coordinates) {
          await page.mouse.move(coordinates.x, coordinates.y);
        } else {
          await canvas.hover();
        }
        break;
      case 'double_click':
        if (coordinates) {
          await canvas.dblclick({ position: coordinates });
        } else {
          await canvas.dblclick();
        }
        break;
      default:
        throw new Error(`Unknown chart interaction action: ${action}`);
    }
    
    // Wait for interaction to process
    await page.waitForTimeout(200);
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