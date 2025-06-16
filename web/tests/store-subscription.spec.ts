import { test, expect } from '@playwright/test';
import type { Page } from '@playwright/test';

/**
 * React Store Subscription Integration Tests
 * 
 * Tests the automatic subscription system between React store and WASM bridge,
 * ensuring real-time synchronization and proper change detection.
 */

class StoreTestHelper {
  constructor(private page: Page) {}

  /**
   * Navigate to app and wait for initialization
   */
  async navigateAndWaitForInit() {
    await this.page.goto('/app');
    
    // Wait for canvas to be visible
    await expect(this.page.locator('#wasm-chart-canvas')).toBeVisible();
    
    // Wait for WASM initialization (check for loading overlay to disappear)
    await this.page.waitForSelector('[data-testid="loading-overlay"]', { state: 'detached', timeout: 15000 });
    
    // Verify chart is initialized (check connection indicator instead)
    await expect(this.page.locator('.bg-green-500')).toBeVisible(); // Connection indicator
  }

  /**
   * Get current store state from the browser
   */
  async getStoreState() {
    return await this.page.evaluate(() => {
      // Access Zustand store directly
      return (window as any).__zustandStore?.getState();
    });
  }

  /**
   * Update store state and wait for synchronization
   */
  async updateStoreState(updates: any) {
    await this.page.evaluate((updates) => {
      const store = (window as any).__zustandStore;
      if (store) {
        if (updates.symbol) {
          store.getState().setCurrentSymbol(updates.symbol);
        }
        if (updates.timeRange) {
          store.getState().setTimeRange(updates.timeRange.startTime, updates.timeRange.endTime);
        }
        if (updates.timeframe) {
          store.getState().setTimeframe(updates.timeframe);
        }
        if (updates.indicators) {
          store.getState().setIndicators(updates.indicators);
        }
      }
    }, updates);

    // Wait for sync to complete (simplified for now)
    await this.page.waitForTimeout(500);
  }

  /**
   * Get WASM chart state for verification
   */
  async getWasmState() {
    return await this.page.evaluate(async () => {
      const chart = (window as any).__wasmChart;
      if (chart && chart.get_current_store_state) {
        const result = await chart.get_current_store_state();
        return result === 'null' ? null : JSON.parse(result);
      }
      return null;
    });
  }

  /**
   * Trigger manual store actions and verify WASM sync
   */
  async triggerStoreAction(action: string, ...args: any[]) {
    return await this.page.evaluate(async ({ action, args }) => {
      const store = (window as any).__zustandStore?.getState();
      if (store && store[action]) {
        store[action](...args);
        
        // Wait a bit for async sync
        await new Promise(resolve => setTimeout(resolve, 200));
        
        return true;
      }
      return false;
    }, { action, args });
  }

  /**
   * Monitor subscription callbacks
   */
  async setupSubscriptionMonitoring() {
    await this.page.evaluate(() => {
      const store = (window as any).__zustandStore?.getState();
      if (store) {
        // Set up monitoring
        (window as any).__subscriptionEvents = [];
        
        store.subscribe('test-monitor', {
          onSymbolChange: (newSymbol, oldSymbol) => {
            (window as any).__subscriptionEvents.push({
              type: 'symbolChange',
              newSymbol,
              oldSymbol,
              timestamp: Date.now()
            });
          },
          onTimeRangeChange: (newRange, oldRange) => {
            (window as any).__subscriptionEvents.push({
              type: 'timeRangeChange',
              newRange,
              oldRange,
              timestamp: Date.now()
            });
          },
          onTimeframeChange: (newTimeframe, oldTimeframe) => {
            (window as any).__subscriptionEvents.push({
              type: 'timeframeChange',
              newTimeframe,
              oldTimeframe,
              timestamp: Date.now()
            });
          },
          onIndicatorsChange: (newIndicators, oldIndicators) => {
            (window as any).__subscriptionEvents.push({
              type: 'indicatorsChange',
              newIndicators,
              oldIndicators,
              timestamp: Date.now()
            });
          },
          onAnyChange: (newState, oldState) => {
            (window as any).__subscriptionEvents.push({
              type: 'anyChange',
              timestamp: Date.now()
            });
          }
        });
      }
    });
  }

  /**
   * Get subscription events
   */
  async getSubscriptionEvents() {
    return await this.page.evaluate(() => {
      return (window as any).__subscriptionEvents || [];
    });
  }

  /**
   * Clear subscription events
   */
  async clearSubscriptionEvents() {
    await this.page.evaluate(() => {
      (window as any).__subscriptionEvents = [];
    });
  }
}

test.describe('React Store Subscription Integration', () => {
  let helper: StoreTestHelper;

  test.beforeEach(async ({ page }) => {
    helper = new StoreTestHelper(page);
    
    // Expose store and chart for testing
    await page.addInitScript(() => {
      (window as any).__zustandStore = null;
      (window as any).__wasmChart = null;
      (window as any).__subscriptionEvents = [];
    });
  });

  test('should initialize store subscription system', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    
    // Verify store is accessible
    const storeState = await helper.getStoreState();
    expect(storeState).toBeTruthy();
    expect(storeState.currentSymbol).toBeTruthy();
    expect(storeState.chartConfig).toBeTruthy();
    
    // Verify WASM chart is initialized
    const wasmState = await helper.getWasmState();
    expect(wasmState).toBeTruthy();
    
    // Verify initial sync
    expect(wasmState.currentSymbol).toBe(storeState.currentSymbol);
    expect(wasmState.chartConfig.symbol).toBe(storeState.chartConfig.symbol);
  });

  test('should automatically sync symbol changes', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    await helper.setupSubscriptionMonitoring();
    
    const initialState = await helper.getStoreState();
    
    // Change symbol
    await helper.updateStoreState({
      symbol: 'ETH-USD'
    });
    
    // Verify React store updated
    const newStoreState = await helper.getStoreState();
    expect(newStoreState.currentSymbol).toBe('ETH-USD');
    expect(newStoreState.chartConfig.symbol).toBe('ETH-USD');
    
    // Verify WASM state synced
    const wasmState = await helper.getWasmState();
    expect(wasmState.currentSymbol).toBe('ETH-USD');
    expect(wasmState.chartConfig.symbol).toBe('ETH-USD');
    
    // Verify subscription callback fired
    const events = await helper.getSubscriptionEvents();
    const symbolChangeEvent = events.find(e => e.type === 'symbolChange');
    expect(symbolChangeEvent).toBeTruthy();
    expect(symbolChangeEvent.newSymbol).toBe('ETH-USD');
    expect(symbolChangeEvent.oldSymbol).toBe(initialState.currentSymbol);
  });

  test('should automatically sync time range changes', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    await helper.setupSubscriptionMonitoring();
    
    const newStartTime = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago
    const newEndTime = Math.floor(Date.now() / 1000);
    
    // Change time range
    await helper.updateStoreState({
      timeRange: {
        startTime: newStartTime,
        endTime: newEndTime
      }
    });
    
    // Verify React store updated
    const storeState = await helper.getStoreState();
    expect(storeState.chartConfig.startTime).toBe(newStartTime);
    expect(storeState.chartConfig.endTime).toBe(newEndTime);
    
    // Verify WASM state synced
    const wasmState = await helper.getWasmState();
    expect(wasmState.chartConfig.startTime).toBe(newStartTime);
    expect(wasmState.chartConfig.endTime).toBe(newEndTime);
    
    // Verify subscription callback fired
    const events = await helper.getSubscriptionEvents();
    const timeRangeEvent = events.find(e => e.type === 'timeRangeChange');
    expect(timeRangeEvent).toBeTruthy();
    expect(timeRangeEvent.newRange.startTime).toBe(newStartTime);
    expect(timeRangeEvent.newRange.endTime).toBe(newEndTime);
  });

  test('should automatically sync timeframe changes', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    await helper.setupSubscriptionMonitoring();
    
    // Change timeframe
    await helper.updateStoreState({
      timeframe: '4h'
    });
    
    // Verify React store updated
    const storeState = await helper.getStoreState();
    expect(storeState.chartConfig.timeframe).toBe('4h');
    
    // Verify WASM state synced
    const wasmState = await helper.getWasmState();
    expect(wasmState.chartConfig.timeframe).toBe('4h');
    
    // Verify subscription callback fired
    const events = await helper.getSubscriptionEvents();
    const timeframeEvent = events.find(e => e.type === 'timeframeChange');
    expect(timeframeEvent).toBeTruthy();
    expect(timeframeEvent.newTimeframe).toBe('4h');
  });

  test('should automatically sync indicator changes', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    await helper.setupSubscriptionMonitoring();
    
    const newIndicators = ['RSI', 'MACD', 'EMA'];
    
    // Change indicators
    await helper.updateStoreState({
      indicators: newIndicators
    });
    
    // Verify React store updated
    const storeState = await helper.getStoreState();
    expect(storeState.chartConfig.indicators).toEqual(newIndicators);
    
    // Verify WASM state synced
    const wasmState = await helper.getWasmState();
    expect(wasmState.chartConfig.indicators).toEqual(newIndicators);
    
    // Verify subscription callback fired
    const events = await helper.getSubscriptionEvents();
    const indicatorsEvent = events.find(e => e.type === 'indicatorsChange');
    expect(indicatorsEvent).toBeTruthy();
    expect(indicatorsEvent.newIndicators).toEqual(newIndicators);
  });

  test('should handle multiple rapid changes with debouncing', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    await helper.setupSubscriptionMonitoring();
    await helper.clearSubscriptionEvents();
    
    // Make multiple rapid changes
    await page.evaluate(async () => {
      const store = (window as any).__zustandStore?.getState();
      if (store) {
        // Rapid symbol changes
        store.setCurrentSymbol('ETH-USD');
        store.setCurrentSymbol('ADA-USD');
        store.setCurrentSymbol('DOT-USD');
        
        // Rapid timeframe changes
        store.setTimeframe('5m');
        store.setTimeframe('15m');
        store.setTimeframe('1h');
        
        // Wait for debouncing to complete
        await new Promise(resolve => setTimeout(resolve, 500));
      }
    });
    
    // Wait for all syncing to complete
    await expect(page.locator('.bg-green-500')).toBeVisible({ timeout: 5000 });
    
    // Verify final state is correct
    const storeState = await helper.getStoreState();
    const wasmState = await helper.getWasmState();
    
    expect(storeState.currentSymbol).toBe('DOT-USD');
    expect(storeState.chartConfig.timeframe).toBe('1h');
    expect(wasmState.currentSymbol).toBe('DOT-USD');
    expect(wasmState.chartConfig.timeframe).toBe('1h');
    
    // Verify subscription events were fired (should have multiple events due to rapid changes)
    const events = await helper.getSubscriptionEvents();
    expect(events.filter(e => e.type === 'symbolChange').length).toBeGreaterThan(0);
    expect(events.filter(e => e.type === 'timeframeChange').length).toBeGreaterThan(0);
  });

  test('should handle store action methods correctly', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    
    // Test addIndicator
    await helper.triggerStoreAction('addIndicator', 'RSI');
    await expect(page.locator('.bg-green-500')).toBeVisible({ timeout: 5000 });
    
    let storeState = await helper.getStoreState();
    let wasmState = await helper.getWasmState();
    expect(storeState.chartConfig.indicators).toContain('RSI');
    expect(wasmState.chartConfig.indicators).toContain('RSI');
    
    // Test removeIndicator
    await helper.triggerStoreAction('removeIndicator', 'RSI');
    await expect(page.locator('.bg-green-500')).toBeVisible({ timeout: 5000 });
    
    storeState = await helper.getStoreState();
    wasmState = await helper.getWasmState();
    expect(storeState.chartConfig.indicators).not.toContain('RSI');
    expect(wasmState.chartConfig.indicators).not.toContain('RSI');
    
    // Test setTimeRange
    const newStartTime = Math.floor(Date.now() / 1000) - 7200; // 2 hours ago
    const newEndTime = Math.floor(Date.now() / 1000);
    
    await helper.triggerStoreAction('setTimeRange', newStartTime, newEndTime);
    await expect(page.locator('.bg-green-500')).toBeVisible({ timeout: 5000 });
    
    storeState = await helper.getStoreState();
    wasmState = await helper.getWasmState();
    expect(storeState.chartConfig.startTime).toBe(newStartTime);
    expect(storeState.chartConfig.endTime).toBe(newEndTime);
    expect(wasmState.chartConfig.startTime).toBe(newStartTime);
    expect(wasmState.chartConfig.endTime).toBe(newEndTime);
  });

  test('should handle batch updates correctly', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    await helper.setupSubscriptionMonitoring();
    await helper.clearSubscriptionEvents();
    
    // Test updateChartState batch operation
    const batchUpdates = {
      symbol: 'AVAX-USD',
      timeframe: '4h',
      indicators: ['RSI', 'MACD']
    };
    
    await helper.triggerStoreAction('updateChartState', batchUpdates);
    await expect(page.locator('.bg-green-500')).toBeVisible({ timeout: 5000 });
    
    // Verify all updates applied
    const storeState = await helper.getStoreState();
    const wasmState = await helper.getWasmState();
    
    expect(storeState.chartConfig.symbol).toBe('AVAX-USD');
    expect(storeState.chartConfig.timeframe).toBe('4h');
    expect(storeState.chartConfig.indicators).toEqual(['RSI', 'MACD']);
    
    expect(wasmState.chartConfig.symbol).toBe('AVAX-USD');
    expect(wasmState.chartConfig.timeframe).toBe('4h');
    expect(wasmState.chartConfig.indicators).toEqual(['RSI', 'MACD']);
    
    // Verify subscription events fired for batch update
    const events = await helper.getSubscriptionEvents();
    expect(events.filter(e => e.type === 'anyChange').length).toBeGreaterThan(0);
  });

  test('should handle error recovery and retry mechanisms', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    
    // Simulate WASM error by breaking the chart instance
    await page.evaluate(() => {
      const chart = (window as any).__wasmChart;
      if (chart) {
        // Break the update method to simulate error
        chart.update_chart_state = () => Promise.reject(new Error('Simulated WASM error'));
      }
    });
    
    // Try to update state (should fail and show error)
    await helper.updateStoreState({ symbol: 'ERROR-TEST' });
    
    // Should show error overlay
    await expect(page.locator('[data-testid="error-overlay"]')).toBeVisible({ timeout: 5000 });
    
    // Click retry button
    await page.locator('[data-testid="retry-button"]').click();
    
    // Should eventually recover (or show that retry was attempted)
    // This test verifies the error handling UI works
  });

  test('should show performance metrics correctly', async ({ page }) => {
    await helper.navigateAndWaitForInit();
    
    // Performance overlay should be visible (top-right corner of canvas)
    const performanceOverlay = page.locator('.absolute.top-4.right-4');
    await expect(performanceOverlay).toBeVisible();
    
    // Should show FPS
    await expect(performanceOverlay).toContainText(/\d+\s+FPS/);
    
    // Make some updates to increment update counter
    await helper.updateStoreState({ symbol: 'ETH-USD' });
    await helper.updateStoreState({ timeframe: '5m' });
    
    // Update counter should increment
    const updateCounter = page.locator('.text-blue-400');
    await expect(updateCounter).toContainText(/#\d+/);
  });

  test('should handle debug mode correctly', async ({ page }) => {
    // Navigate with debug mode enabled
    await page.goto('/app?debug=true');
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible();
    
    // Wait for initialization
    await page.waitForSelector('[data-testid="loading-overlay"]', { state: 'detached', timeout: 15000 });
    
    // Debug panel should be visible (top-left corner of canvas)
    const debugPanel = page.locator('.absolute.top-4.left-4');
    await expect(debugPanel).toBeVisible();
    
    // Should show debug information
    await expect(debugPanel).toContainText('Initialized:');
    await expect(debugPanel).toContainText('Loading:');
    await expect(debugPanel).toContainText('Error:');
    await expect(debugPanel).toContainText('Changes:');
    
    // Test force update button
    await page.locator('[data-testid="force-update-button"]').click();
    
    // Test get state button
    await page.locator('[data-testid="get-state-button"]').click();
    
    // Should log to console (we can't easily test console output in Playwright,
    // but we can verify the buttons are clickable and don't error)
  });
});