import { test, expect } from '@playwright/test';
import { GraphTestUtils } from './helpers/test-utils';

test.describe('Simple Data Visualization Tests', () => {
  let utils: GraphTestUtils;

  test.beforeEach(async ({ page }) => {
    utils = new GraphTestUtils(page);
  });

  test('should load app and display chart canvas', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();

    // Basic check - canvas should be visible and properly sized
    const canvas = page.locator('#new-api-canvas');
    await expect(canvas).toBeVisible();

    const canvasBox = await canvas.boundingBox();
    expect(canvasBox?.width).toBeGreaterThan(300);
    expect(canvasBox?.height).toBeGreaterThan(200);
  });

  test('should handle user interactions without crashing', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();

    const canvas = page.locator('#new-api-canvas');
    
    // Test basic interactions
    await canvas.hover();
    await page.mouse.wheel(0, -100); // Zoom in
    await page.waitForTimeout(300);
    
    await page.mouse.wheel(0, 100); // Zoom out  
    await page.waitForTimeout(300);

    // Canvas should still be visible after interactions
    await expect(canvas).toBeVisible();
  });

  test('should handle different viewport sizes', async ({ page }) => {
    const viewports = [
      { width: 1920, height: 1080 },
      { width: 1366, height: 768 }, 
      { width: 768, height: 1024 }
    ];

    for (const viewport of viewports) {
      await page.setViewportSize(viewport);
      await utils.navigateToApp();
      await utils.waitForChartRender();

      const canvas = page.locator('#new-api-canvas');
      await expect(canvas).toBeVisible();

      const canvasBox = await canvas.boundingBox();
      expect(canvasBox?.width).toBeGreaterThan(200);
      expect(canvasBox?.height).toBeGreaterThan(150);
      
      console.log(`✓ Tested viewport: ${viewport.width}x${viewport.height}`);
    }
  });

  test('should show loading state initially', async ({ page }) => {
    await page.goto('/app');

    // Should show loading state first
    const loadingText = page.getByText('Loading Chart Engine');
    await expect(loadingText).toBeVisible({ timeout: 5000 });
    
    // Eventually should show canvas
    const canvas = page.locator('#new-api-canvas');
    await expect(canvas).toBeVisible({ timeout: 20000 });
  });

  test('should maintain performance during sustained use', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();

    const canvas = page.locator('#new-api-canvas');
    const canvasBox = await utils.getCanvasDimensions();
    
    if (!canvasBox) return;

    const startTime = Date.now();
    
    // Perform sustained interactions
    for (let i = 0; i < 20; i++) {
      await utils.zoomChart(-50);
      await utils.zoomChart(50);
      
      if (i % 5 === 0) {
        console.log(`Completed ${i + 1}/20 interactions`);
      }
    }
    
    const totalTime = Date.now() - startTime;
    console.log(`Total interaction time: ${totalTime}ms`);
    
    // Should complete in reasonable time (less than 30 seconds)
    expect(totalTime).toBeLessThan(30000);
    
    // Canvas should still be responsive
    await expect(canvas).toBeVisible();
  });

  test('should handle rapid interactions gracefully', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();

    const canvas = page.locator('#new-api-canvas');
    
    // Rapid fire interactions
    for (let i = 0; i < 10; i++) {
      await canvas.hover();
      await page.mouse.wheel(0, i % 2 === 0 ? -30 : 30);
      // No wait between interactions - test rapid fire
    }
    
    // Give time for all interactions to complete
    await page.waitForTimeout(1000);
    
    // Should still be functional
    await expect(canvas).toBeVisible();
    
    // Should be able to perform normal interaction after rapid fire
    await utils.zoomChart(-100);
    await expect(canvas).toBeVisible();
  });

  test('should handle window resize gracefully', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();

    const canvas = page.locator('#new-api-canvas');
    
    // Get initial canvas size
    const initialBox = await canvas.boundingBox();
    
    // Resize window
    await page.setViewportSize({ width: 1600, height: 900 });
    await page.waitForTimeout(1000);
    
    // Canvas should adapt to new size
    const resizedBox = await canvas.boundingBox();
    expect(resizedBox?.width).toBeGreaterThan(200);
    expect(resizedBox?.height).toBeGreaterThan(150);
    
    // Canvas should still be functional
    await utils.zoomChart(-100);
    await expect(canvas).toBeVisible();
  });

  test('should not have memory leaks during normal use', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();

    // Get initial memory if available
    const initialMemory = await page.evaluate(() => {
      return (performance as any).memory?.usedJSHeapSize || null;
    });

    if (!initialMemory) {
      test.skip('Memory API not available in this browser');
      return;
    }

    const canvas = page.locator('#new-api-canvas');
    const canvasBox = await utils.getCanvasDimensions();
    
    if (!canvasBox) return;

    // Perform normal usage pattern
    for (let i = 0; i < 10; i++) {
      await utils.zoomChart(-100);
      await utils.zoomChart(100);
      await utils.panChart(
        canvasBox.x + 100, 
        canvasBox.y + 100,
        canvasBox.x + 200, 
        canvasBox.y + 100
      );
    }

    // Force garbage collection if available
    await page.evaluate(() => {
      if ('gc' in window) {
        (window as any).gc();
      }
    });

    await page.waitForTimeout(2000);

    const finalMemory = await page.evaluate(() => {
      return (performance as any).memory?.usedJSHeapSize || null;
    });

    if (finalMemory && initialMemory) {
      const memoryGrowth = finalMemory - initialMemory;
      const growthPercentage = (memoryGrowth / initialMemory) * 100;
      
      console.log(`Memory growth: ${(memoryGrowth / 1024 / 1024).toFixed(2)}MB (${growthPercentage.toFixed(1)}%)`);
      
      // Memory should not grow excessively (allow up to 100% growth)
      expect(growthPercentage).toBeLessThan(100);
    }
  });

  test('should handle error states gracefully', async ({ page }) => {
    // Test with a route that might cause issues
    await page.goto('/app/nonexistent');
    
    // Should either redirect to working state or show error gracefully
    await page.waitForTimeout(3000);
    
    // Should not show browser error page
    const bodyText = await page.textContent('body');
    expect(bodyText).not.toContain('404');
    expect(bodyText).not.toContain('Cannot GET');
    
    // Should eventually show some UI (canvas or error state)
    const hasCanvas = await page.locator('#new-api-canvas').isVisible();
    const hasError = await page.locator('[data-testid="error-overlay"]').isVisible();
    const hasContent = await page.locator('#root').isVisible();
    
    expect(hasCanvas || hasError || hasContent).toBe(true);
  });
});