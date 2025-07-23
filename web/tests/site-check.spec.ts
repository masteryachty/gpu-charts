import { test, expect } from '@playwright/test';

test.describe('Site Functionality', () => {
  test('homepage loads without errors', async ({ page }) => {
    // Monitor console errors
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    // Navigate to homepage
    await page.goto('/');
    
    // Wait for content to load
    await page.waitForLoadState('networkidle');
    
    // Check no errors occurred
    expect(errors).toHaveLength(0);
    
    // Check page has rendered
    await expect(page.locator('h1')).toBeVisible();
  });

  test('trading app loads without module errors', async ({ page }) => {
    // Monitor console and network errors
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });
    
    page.on('pageerror', error => {
      errors.push(error.message);
    });

    // Navigate to trading app
    await page.goto('/app');
    
    // Wait for initial load
    await page.waitForLoadState('networkidle');
    
    // Check no errors
    console.log('Console errors:', errors);
    expect(errors).toHaveLength(0);
    
    // Check key components are rendered
    await expect(page.locator('text=Trading Dashboard')).toBeVisible();
    await expect(page.locator('#wasm-chart-canvas')).toBeVisible();
  });

  test('WASM module loads successfully', async ({ page }) => {
    const errors: string[] = [];
    const logs: string[] = [];
    
    page.on('console', msg => {
      const text = msg.text();
      if (msg.type() === 'error') {
        errors.push(text);
      } else if (text.includes('WASM')) {
        logs.push(text);
      }
    });

    await page.goto('/app');
    
    // Wait for WASM to initialize
    await page.waitForTimeout(3000);
    
    // Check WASM logs
    console.log('WASM logs:', logs);
    
    // Verify canvas is initialized
    const canvasInitialized = await page.evaluate(() => {
      const canvas = document.getElementById('wasm-chart-canvas');
      return canvas !== null && canvas instanceof HTMLCanvasElement;
    });
    
    expect(canvasInitialized).toBe(true);
    
    // Check no WASM errors
    const wasmErrors = errors.filter(e => e.includes('wasm') || e.includes('WASM'));
    expect(wasmErrors).toHaveLength(0);
  });

  test('chart controls are interactive', async ({ page }) => {
    await page.goto('/app');
    await page.waitForLoadState('networkidle');
    
    // Test chart type switcher
    const lineButton = page.locator('button:has-text("Line")');
    const candlestickButton = page.locator('button:has-text("Candlestick")');
    
    await expect(lineButton).toBeVisible();
    await expect(candlestickButton).toBeVisible();
    
    // Click candlestick
    await candlestickButton.click();
    
    // Verify candlestick is selected (has different styling)
    await expect(candlestickButton).toHaveClass(/bg-blue-600/);
    
    // Verify timeframe selector appears
    const timeframeSelect = page.locator('[data-testid="timeframe-select"]');
    await expect(timeframeSelect).toBeVisible();
  });

  test('sidebar is functional', async ({ page }) => {
    await page.goto('/app');
    await page.waitForLoadState('networkidle');
    
    // Check sidebar exists
    const sidebar = page.locator('text=Tools').first();
    await expect(sidebar).toBeVisible();
    
    // Test collapse button
    const collapseButton = page.locator('button:has(svg)').first();
    await collapseButton.click();
    
    // Verify sidebar collapsed
    await expect(page.locator('text=Tools')).not.toBeVisible();
  });
});