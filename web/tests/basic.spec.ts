import { test, expect } from '@playwright/test';

test.describe('Basic App Functionality', () => {
  
  test('should load the homepage', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveTitle(/Graph/i);
    await expect(page.locator('#root')).toBeVisible();
  });

  test('should navigate to trading app', async ({ page }) => {
    await page.goto('/app');
    
    // Should show the trading app layout
    await expect(page.locator('#root')).toBeVisible();
    
    // Should have canvas element eventually
    await expect(page.locator('#new-api-canvas')).toBeVisible({ timeout: 20000 });
  });

  test('should show loading state initially', async ({ page }) => {
    await page.goto('/app');
    
    // Should show loading initially
    const loadingText = page.getByText('Loading Chart Engine');
    await expect(loadingText).toBeVisible({ timeout: 5000 });
  });

  test('should handle canvas rendering', async ({ page }) => {
    await page.goto('/app');
    
    // Wait for canvas to appear
    const canvas = page.locator('#new-api-canvas');
    await expect(canvas).toBeVisible({ timeout: 20000 });
    
    // Canvas should have dimensions
    const box = await canvas.boundingBox();
    expect(box?.width).toBeGreaterThan(100);
    expect(box?.height).toBeGreaterThan(100);
  });

  test('should handle basic interactions', async ({ page }) => {
    await page.goto('/app');
    
    // Wait for canvas
    const canvas = page.locator('#new-api-canvas');
    await expect(canvas).toBeVisible({ timeout: 20000 });
    
    // Try hovering on canvas (should not crash)
    await canvas.hover();
    
    // Try scrolling (should not crash)
    await canvas.hover();
    await page.mouse.wheel(0, -100);
    await page.waitForTimeout(500);
    
    // Canvas should still be visible
    await expect(canvas).toBeVisible();
  });
});