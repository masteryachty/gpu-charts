import { Page, expect } from '@playwright/test';
import { DataMockHelper } from './data-mocks';
import { GraphTestUtils } from './test-utils';

/**
 * Specialized Test Utilities for React-Rust Integration Testing
 * 
 * Enhanced utilities specifically designed for testing the complete
 * React Store → Rust integration system.
 */

export class IntegrationTestUtils {
  
  /**
   * Wait for complete system initialization
   */
  static async waitForSystemReady(page: Page, timeout = 15000): Promise<void> {
    await Promise.all([
      // Wait for WASM chart initialization
      this.waitForWasmReady(page, timeout),
      
      // Wait for store initialization
      this.waitForStoreReady(page, timeout),
      
      // Wait for data fetching service ready
      this.waitForDataServiceReady(page, timeout),
      
      // Wait for error handler ready
      this.waitForErrorHandlerReady(page, timeout),
      
      // Wait for performance monitor ready
      this.waitForPerformanceMonitorReady(page, timeout)
    ]);
  }
  
  /**
   * Wait for WASM chart to be fully initialized
   */
  static async waitForWasmReady(page: Page, timeout = 10000): Promise<void> {
    await page.waitForFunction(() => {
      const canvas = document.getElementById('wasm-chart-canvas');
      return canvas && (window as any).__WASM_CHART_READY__ === true;
    }, { timeout });
  }
  
  /**
   * Wait for React store to be initialized
   */
  static async waitForStoreReady(page: Page, timeout = 5000): Promise<void> {
    await page.waitForFunction(() => {
      return (window as any).__APP_STORE_STATE__ && 
             (window as any).__STORE_READY__ === true;
    }, { timeout });
  }
  
  /**
   * Wait for data fetching service to be ready
   */
  static async waitForDataServiceReady(page: Page, timeout = 5000): Promise<void> {
    await page.waitForFunction(() => {
      return (window as any).__DATA_SERVICE_READY__ === true;
    }, { timeout });
  }
  
  /**
   * Wait for error handler to be initialized
   */
  static async waitForErrorHandlerReady(page: Page, timeout = 5000): Promise<void> {
    await page.waitForFunction(() => {
      return (window as any).__ERROR_HANDLER_READY__ === true;
    }, { timeout });
  }
  
  /**
   * Wait for performance monitor to be active
   */
  static async waitForPerformanceMonitorReady(page: Page, timeout = 5000): Promise<void> {
    await page.waitForFunction(() => {
      return (window as any).__PERFORMANCE_MONITOR_READY__ === true;
    }, { timeout });
  }
  
  /**
   * Inject test hooks into the page for easier testing
   */
  static async injectTestHooks(page: Page): Promise<void> {
    await page.addInitScript(() => {
      // Global test state
      (window as any).__TEST_HOOKS__ = {
        wasmUpdateCount: 0,
        storeUpdateCount: 0,
        dataFetchCount: 0,
        errorCount: 0,
        lastChangeDetection: null,
        performanceMetrics: null
      };
      
      // Hook into WASM updates
      const originalWasmUpdate = (window as any).__UPDATE_WASM_CHART_STATE__;
      if (originalWasmUpdate) {
        (window as any).__UPDATE_WASM_CHART_STATE__ = function(...args: any[]) {
          (window as any).__TEST_HOOKS__.wasmUpdateCount++;
          return originalWasmUpdate.apply(this, args);
        };
      }
      
      // Hook into store updates
      const originalStoreUpdate = (window as any).__UPDATE_STORE_STATE__;
      if (originalStoreUpdate) {
        (window as any).__UPDATE_STORE_STATE__ = function(...args: any[]) {
          (window as any).__TEST_HOOKS__.storeUpdateCount++;
          return originalStoreUpdate.apply(this, args);
        };
      }
      
      // Hook into data fetches
      const originalFetch = window.fetch;
      window.fetch = function(...args: any[]) {
        if (args[0] && typeof args[0] === 'string' && args[0].includes('/api/data')) {
          (window as any).__TEST_HOOKS__.dataFetchCount++;
        }
        return originalFetch.apply(this, args);
      };
      
      // Hook into error reports
      const originalErrorHandler = (window as any).__REPORT_ERROR__;
      if (originalErrorHandler) {
        (window as any).__REPORT_ERROR__ = function(...args: any[]) {
          (window as any).__TEST_HOOKS__.errorCount++;
          return originalErrorHandler.apply(this, args);
        };
      }
    });
  }
  
  /**
   * Get comprehensive test metrics
   */
  static async getTestMetrics(page: Page): Promise<{
    wasmUpdateCount: number;
    storeUpdateCount: number;
    dataFetchCount: number;
    errorCount: number;
    performanceScore: number;
  }> {
    return await page.evaluate(() => {
      const hooks = (window as any).__TEST_HOOKS__ || {};
      const performanceMetrics = (window as any).__PERFORMANCE_METRICS__ || {};
      
      // Calculate simple performance score
      const fps = performanceMetrics.fps || 60;
      const memory = performanceMetrics.totalMemoryUsage || 0;
      const latency = performanceMetrics.renderLatency || 0;
      
      let score = 100;
      if (fps < 30) score -= 30;
      if (memory > 500 * 1024 * 1024) score -= 20;
      if (latency > 50) score -= 25;
      
      return {
        wasmUpdateCount: hooks.wasmUpdateCount || 0,
        storeUpdateCount: hooks.storeUpdateCount || 0,
        dataFetchCount: hooks.dataFetchCount || 0,
        errorCount: hooks.errorCount || 0,
        performanceScore: Math.max(score, 0)
      };
    });
  }
  
  /**
   * Simulate realistic user interaction patterns
   */
  static async simulateUserWorkflow(page: Page, workflowType: 'basic' | 'complex' | 'stress'): Promise<void> {
    switch (workflowType) {
      case 'basic':
        await this.simulateBasicWorkflow(page);
        break;
      case 'complex':
        await this.simulateComplexWorkflow(page);
        break;
      case 'stress':
        await this.simulateStressWorkflow(page);
        break;
    }
  }
  
  private static async simulateBasicWorkflow(page: Page): Promise<void> {
    // Basic workflow: change symbol, change timeframe, zoom chart
    await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
    await page.waitForTimeout(500);
    
    await page.selectOption('select[data-testid="timeframe-selector"]', '1h');
    await page.waitForTimeout(500);
    
    // Zoom interaction
    const canvas = page.locator('#wasm-chart-canvas');
    await canvas.hover();
    await page.mouse.wheel(0, -100);
    await page.waitForTimeout(300);
  }
  
  private static async simulateComplexWorkflow(page: Page): Promise<void> {
    // Complex workflow: multiple symbol changes, indicator toggles, performance monitoring
    const symbols = ['BTC-USD', 'ETH-USD', 'ADA-USD', 'DOT-USD'];
    const timeframes = ['1m', '5m', '15m', '1h'];
    
    for (let i = 0; i < symbols.length; i++) {
      await page.selectOption('select[data-testid="symbol-selector"]', symbols[i]);
      await page.waitForTimeout(200);
      
      await page.selectOption('select[data-testid="timeframe-selector"]', timeframes[i]);
      await page.waitForTimeout(200);
      
      // Chart interactions
      const canvas = page.locator('#wasm-chart-canvas');
      await canvas.hover();
      
      // Zoom in and out
      await page.mouse.wheel(0, -50);
      await page.waitForTimeout(100);
      await page.mouse.wheel(0, 50);
      await page.waitForTimeout(100);
      
      // Pan chart
      await canvas.click({ position: { x: 300, y: 200 } });
      await page.mouse.move(400, 200);
      await page.waitForTimeout(100);
    }
  }
  
  private static async simulateStressWorkflow(page: Page): Promise<void> {
    // Stress test: rapid changes to test debouncing and performance
    const symbols = ['BTC-USD', 'ETH-USD', 'ADA-USD'];
    
    // Rapid symbol changes
    for (let i = 0; i < 20; i++) {
      await page.selectOption('select[data-testid="symbol-selector"]', symbols[i % symbols.length]);
      await page.waitForTimeout(50);
    }
    
    // Rapid chart interactions
    const canvas = page.locator('#wasm-chart-canvas');
    await canvas.hover();
    
    for (let i = 0; i < 30; i++) {
      await page.mouse.wheel(0, i % 2 === 0 ? -25 : 25);
      await page.waitForTimeout(20);
    }
    
    // Wait for systems to settle
    await page.waitForTimeout(1000);
  }
  
  /**
   * Verify system consistency across all components
   */
  static async verifySystemConsistency(page: Page, expectedState: {
    symbol: string;
    timeframe: string;
  }): Promise<void> {
    // Get state from all systems
    const states = await page.evaluate(() => ({
      store: (window as any).__APP_STORE_STATE__,
      wasm: (window as any).__GET_WASM_CHART_STATE__?.(),
      dataService: (window as any).__DATA_SERVICE_STATE__,
      ui: {
        symbol: document.querySelector('[data-testid="current-symbol"]')?.textContent,
        timeframe: (document.querySelector('[data-testid="timeframe-selector"]') as HTMLSelectElement)?.value
      }
    }));
    
    // Verify consistency
    expect(states.store.currentSymbol).toBe(expectedState.symbol);
    expect(states.store.chartConfig.timeframe).toBe(expectedState.timeframe);
    
    if (states.wasm) {
      expect(states.wasm.currentSymbol).toBe(expectedState.symbol);
      expect(states.wasm.chartConfig.timeframe).toBe(expectedState.timeframe);
    }
    
    expect(states.ui.symbol).toContain(expectedState.symbol);
    expect(states.ui.timeframe).toBe(expectedState.timeframe);
  }
  
  /**
   * Test error recovery scenarios
   */
  static async testErrorRecovery(page: Page, errorType: 'wasm' | 'data' | 'network' | 'validation'): Promise<void> {
    switch (errorType) {
      case 'wasm':
        await this.testWasmErrorRecovery(page);
        break;
      case 'data':
        await this.testDataErrorRecovery(page);
        break;
      case 'network':
        await this.testNetworkErrorRecovery(page);
        break;
      case 'validation':
        await this.testValidationErrorRecovery(page);
        break;
    }
  }
  
  private static async testWasmErrorRecovery(page: Page): Promise<void> {
    // Simulate WASM error
    await page.evaluate(() => {
      (window as any).__FORCE_WASM_ERROR__ = true;
    });
    
    // Trigger WASM operation
    await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
    
    // Should show error and recovery options
    await expect(page.locator(':text("Try Again")')).toBeVisible({ timeout: 3000 });
    
    // Clear error simulation
    await page.evaluate(() => {
      (window as any).__FORCE_WASM_ERROR__ = false;
    });
    
    // Attempt recovery
    await page.locator(':text("Try Again")').click();
    
    // Should eventually recover
    await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 5000 });
  }
  
  private static async testDataErrorRecovery(page: Page): Promise<void> {
    // Mock server error
    await page.route('**/api/data**', route => route.fulfill({ status: 500 }));
    
    // Trigger data fetch
    await page.selectOption('select[data-testid="symbol-selector"]', 'TEST-USD');
    
    // Should show error notification
    await expect(page.locator('.bg-red-900')).toBeVisible({ timeout: 3000 });
    
    // Clear route mock
    await page.unroute('**/api/data**');
    
    // Should automatically retry and recover
    await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 10000 });
  }
  
  private static async testNetworkErrorRecovery(page: Page): Promise<void> {
    // Simulate offline
    await page.context().setOffline(true);
    
    // Try to change symbol
    await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
    
    // Should show network error
    await expect(page.locator(':text("connection")')).toBeVisible({ timeout: 3000 });
    
    // Restore network
    await page.context().setOffline(false);
    
    // Should recover automatically
    await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 5000 });
  }
  
  private static async testValidationErrorRecovery(page: Page): Promise<void> {
    // Trigger validation error
    await page.evaluate(() => {
      (window as any).__FORCE_VALIDATION_ERROR__ = {
        symbol: '',
        timeframe: 'invalid'
      };
    });
    
    // Should show validation error
    await expect(page.locator('.bg-red-900')).toBeVisible({ timeout: 3000 });
    
    // Clear validation error
    await page.evaluate(() => {
      (window as any).__FORCE_VALIDATION_ERROR__ = null;
    });
    
    // Make valid change
    await page.selectOption('select[data-testid="symbol-selector"]', 'BTC-USD');
    
    // Should recover
    await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 3000 });
  }
  
  /**
   * Measure and verify performance under load
   */
  static async measurePerformanceUnderLoad(page: Page, loadType: 'cpu' | 'memory' | 'network'): Promise<{
    baselinePerformance: any;
    loadPerformance: any;
    degradation: number;
  }> {
    // Measure baseline performance
    await page.waitForTimeout(1000);
    const baselinePerformance = await page.evaluate(() => (window as any).__PERFORMANCE_METRICS__);
    
    // Apply load
    switch (loadType) {
      case 'cpu':
        await this.applyCpuLoad(page);
        break;
      case 'memory':
        await this.applyMemoryLoad(page);
        break;
      case 'network':
        await this.applyNetworkLoad(page);
        break;
    }
    
    // Measure performance under load
    await page.waitForTimeout(2000);
    const loadPerformance = await page.evaluate(() => (window as any).__PERFORMANCE_METRICS__);
    
    // Calculate degradation
    const degradation = this.calculatePerformanceDegradation(baselinePerformance, loadPerformance);
    
    return { baselinePerformance, loadPerformance, degradation };
  }
  
  private static async applyCpuLoad(page: Page): Promise<void> {
    await page.evaluate(() => {
      // Create CPU-intensive task
      const startTime = Date.now();
      while (Date.now() - startTime < 2000) {
        Math.random() * Math.random();
      }
    });
  }
  
  private static async applyMemoryLoad(page: Page): Promise<void> {
    await page.evaluate(() => {
      // Create memory pressure
      const arrays = [];
      for (let i = 0; i < 500; i++) {
        arrays.push(new Array(50000).fill(Math.random()));
      }
      (window as any).__MEMORY_LOAD__ = arrays;
    });
  }
  
  private static async applyNetworkLoad(page: Page): Promise<void> {
    // Throttle network
    await page.context().setExtraHTTPHeaders({
      'X-Throttle-Network': 'true'
    });
  }
  
  private static calculatePerformanceDegradation(baseline: any, loaded: any): number {
    if (!baseline || !loaded) return 0;
    
    const fpsChange = (baseline.fps - loaded.fps) / baseline.fps * 100;
    const memoryChange = (loaded.totalMemoryUsage - baseline.totalMemoryUsage) / baseline.totalMemoryUsage * 100;
    const latencyChange = (loaded.renderLatency - baseline.renderLatency) / baseline.renderLatency * 100;
    
    return Math.max(fpsChange, memoryChange * 0.5, latencyChange);
  }
}

/**
 * Enhanced Data Mock Helper for Integration Testing
 */
export class IntegrationDataMockHelper extends DataMockHelper {
  
  /**
   * Mock real-time data stream
   */
  static async mockRealTimeDataStream(page: Page, symbol: string, durationMs: number): Promise<void> {
    const interval = 1000; // Update every second
    const updates = Math.floor(durationMs / interval);
    
    for (let i = 0; i < updates; i++) {
      const mockData = this.generateMarketData(symbol, 1);
      await this.mockServerResponse(page, mockData);
      await page.waitForTimeout(interval);
    }
  }
  
  /**
   * Mock progressive data loading (simulating large dataset)
   */
  static async mockProgressiveDataLoading(page: Page, symbol: string, totalRecords: number): Promise<void> {
    const chunkSize = 1000;
    const chunks = Math.ceil(totalRecords / chunkSize);
    
    for (let i = 0; i < chunks; i++) {
      const recordsInChunk = Math.min(chunkSize, totalRecords - i * chunkSize);
      const mockData = this.generateMarketData(symbol, recordsInChunk);
      
      await this.mockServerResponse(page, mockData);
      
      // Simulate loading delay
      await page.waitForTimeout(200);
    }
  }
  
  /**
   * Mock various error scenarios
   */
  static async mockErrorScenarios(page: Page, scenario: 'timeout' | 'server_error' | 'invalid_data' | 'partial_failure'): Promise<void> {
    switch (scenario) {
      case 'timeout':
        await page.route('**/api/data**', async (route) => {
          // Delay response to trigger timeout
          await new Promise(resolve => setTimeout(resolve, 10000));
          await route.fulfill({ status: 200, body: '{}' });
        });
        break;
        
      case 'server_error':
        await page.route('**/api/data**', route => 
          route.fulfill({ status: 500, body: 'Internal Server Error' })
        );
        break;
        
      case 'invalid_data':
        await page.route('**/api/data**', route => 
          route.fulfill({ status: 200, body: 'invalid json data' })
        );
        break;
        
      case 'partial_failure':
        let requestCount = 0;
        await page.route('**/api/data**', route => {
          requestCount++;
          if (requestCount % 3 === 0) {
            route.fulfill({ status: 500, body: 'Intermittent Error' });
          } else {
            route.fulfill({ status: 200, body: JSON.stringify(this.generateMarketData('BTC-USD', 100)) });
          }
        });
        break;
    }
  }
}

// Re-export utilities for convenience
export { DataMockHelper } from './data-mocks';
export { GraphTestUtils } from './test-utils';