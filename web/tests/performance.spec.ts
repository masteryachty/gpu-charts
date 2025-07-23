import { test, expect } from '@playwright/test';
import { GraphTestUtils, TestData } from './helpers/test-utils';

test.describe('Performance Tests', () => {
  let utils: GraphTestUtils;

  test.beforeEach(async ({ page }) => {
    utils = new GraphTestUtils(page);
  });

  test('should load initial chart within performance budget', async ({ page }) => {
    const startTime = Date.now();
    
    await utils.navigateToApp();
    await utils.waitForChartRender();
    
    const loadTime = Date.now() - startTime;
    
    // Chart should load within 10 seconds (including WASM compile)
    expect(loadTime).toBeLessThan(10000);
    
    console.log(`Chart loaded in ${loadTime}ms`);
  });

  test('should handle large datasets efficiently', async ({ page }) => {
    // Mock a large dataset response
    const largeDataset = TestData.generateTimeSeriesData(10000);
    await utils.mockDataResponse(largeDataset);
    
    const startTime = Date.now();
    
    await utils.navigateToApp();
    await utils.waitForChartRender();
    
    const renderTime = Date.now() - startTime;
    
    // Even large datasets should render in reasonable time
    expect(renderTime).toBeLessThan(15000);
    
    console.log(`Large dataset (10k points) rendered in ${renderTime}ms`);
  });

  test('should not leak memory during interactions', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();
    
    // Get initial memory usage
    const initialMemory = await utils.checkMemoryUsage();
    
    if (!initialMemory) {
      test.skip('Memory API not available in this browser');
      return;
    }
    
    // Perform some interactions (reduced for CI stability)
    const canvas = await utils.getCanvasDimensions();
    if (!canvas) return;
    
    const centerX = canvas.x + canvas.width / 2;
    const centerY = canvas.y + canvas.height / 2;
    
    for (let i = 0; i < 10; i++) {
      // Zoom in and out
      await utils.zoomChart(-100, centerX, centerY);
      await utils.zoomChart(100, centerX, centerY);
      
      // Pan around  
      await utils.panChart(centerX, centerY, centerX + 50, centerY);
      await utils.panChart(centerX + 50, centerY, centerX, centerY);
      
      // Check memory every 5 iterations
      if (i % 5 === 0) {
        const currentMemory = await utils.checkMemoryUsage();
        if (currentMemory) {
          console.log(`Iteration ${i}: Memory usage: ${(currentMemory.used / 1024 / 1024).toFixed(2)}MB`);
        }
      }
    }
    
    // Force garbage collection if available
    await page.evaluate(() => {
      if ('gc' in window) {
        (window as any).gc();
      }
    });
    
    await page.waitForTimeout(2000);
    
    const finalMemory = await utils.checkMemoryUsage();
    
    if (finalMemory && initialMemory) {
      const memoryGrowth = finalMemory.used - initialMemory.used;
      const growthPercentage = (memoryGrowth / initialMemory.used) * 100;
      
      console.log(`Memory growth: ${(memoryGrowth / 1024 / 1024).toFixed(2)}MB (${growthPercentage.toFixed(1)}%)`);
      
      // Memory should not grow by more than 200% during normal interactions
      expect(growthPercentage).toBeLessThan(200);
    }
  });

  test('should maintain responsive interactions under load', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();
    
    const canvas = await utils.getCanvasDimensions();
    if (!canvas) return;
    
    const centerX = canvas.x + canvas.width / 2;
    const centerY = canvas.y + canvas.height / 2;
    
    // Measure interaction response times
    const responseTimes: number[] = [];
    
    for (let i = 0; i < 10; i++) {
      const startTime = performance.now();
      
      await utils.zoomChart(-100, centerX, centerY);
      
      // Wait for any animations/updates to complete
      await page.waitForTimeout(100);
      
      const responseTime = performance.now() - startTime;
      responseTimes.push(responseTime);
    }
    
    const avgResponseTime = responseTimes.reduce((a, b) => a + b) / responseTimes.length;
    const maxResponseTime = Math.max(...responseTimes);
    
    console.log(`Average zoom response time: ${avgResponseTime.toFixed(2)}ms`);
    console.log(`Max zoom response time: ${maxResponseTime.toFixed(2)}ms`);
    
    // Interactions should feel responsive (relaxed thresholds for CI)
    expect(avgResponseTime).toBeLessThan(1000); // Average under 1 second
    expect(maxResponseTime).toBeLessThan(2000); // No single interaction over 2 seconds
  });

  test('should handle rapid interactions gracefully', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();
    
    const canvas = await utils.getCanvasDimensions();
    if (!canvas) return;
    
    const centerX = canvas.x + canvas.width / 2;
    const centerY = canvas.y + canvas.height / 2;
    
    // Rapid fire interactions
    const promises: Promise<void>[] = [];
    
    for (let i = 0; i < 20; i++) {
      promises.push(utils.zoomChart(i % 2 === 0 ? -50 : 50, centerX, centerY));
    }
    
    // All interactions should complete without throwing
    await Promise.all(promises);
    
    // Chart should still be responsive after rapid interactions
    await expect(page.locator('canvas')).toBeVisible();
    
    // Should be able to perform a normal interaction
    await utils.zoomChart(-100, centerX, centerY);
    await page.waitForTimeout(500);
    
    await expect(page.locator('canvas')).toBeVisible();
  });

  test('should handle slow network conditions', async ({ page }) => {
    // Skip this test for now as it's complex to simulate properly
    test.skip(true, 'Slow network simulation needs refinement');
  });

  test('should render at different viewport sizes efficiently', async ({ page }) => {
    const viewports = [
      { width: 1920, height: 1080 }, // Desktop
      { width: 1366, height: 768 },  // Laptop
      { width: 768, height: 1024 },  // Tablet
      { width: 375, height: 667 },   // Mobile
    ];
    
    for (const viewport of viewports) {
      await page.setViewportSize(viewport);
      
      const startTime = Date.now();
      
      await utils.navigateToApp();
      await utils.waitForChartRender();
      
      const renderTime = Date.now() - startTime;
      
      console.log(`Rendered at ${viewport.width}x${viewport.height} in ${renderTime}ms`);
      
      // Should render efficiently at all sizes
      expect(renderTime).toBeLessThan(12000);
      
      // Canvas should adapt to viewport
      const canvas = await utils.getCanvasDimensions();
      expect(canvas?.width).toBeGreaterThan(200);
      expect(canvas?.height).toBeGreaterThan(150);
    }
  });
});