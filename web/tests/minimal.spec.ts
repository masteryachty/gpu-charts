import { test, expect } from '@playwright/test';

/**
 * Minimal smoke tests to verify core functionality
 * These tests focus on essential functionality without depending on CSS visibility
 */

test.describe('Minimal App Smoke Tests', () => {
  
  test('app loads without JavaScript errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('/');
    await page.waitForTimeout(3000);
    
    // Should load with correct title
    await expect(page).toHaveTitle(/Graph/i);
    
    // Should have some DOM content
    const hasContent = await page.evaluate(() => {
      return document.body.children.length > 0;
    });
    expect(hasContent).toBe(true);
    
    // Should not have critical JavaScript errors
    const criticalErrors = errors.filter(error => 
      !error.includes('favicon') && 
      !error.includes('WebGPU') &&
      !error.includes('Warning')
    );
    expect(criticalErrors.length).toBe(0);
  });

  test('trading app route loads', async ({ page }) => {
    const errors: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('/app');
    await page.waitForTimeout(5000);
    
    // Should have DOM content
    const hasContent = await page.evaluate(() => {
      return document.body.children.length > 0;
    });
    expect(hasContent).toBe(true);
    
    // Should have either canvas or some chart-related content
    const hasChartContent = await page.evaluate(() => {
      const canvas = document.getElementById('wasm-chart-canvas');
      const bodyText = document.body.textContent || '';
      return !!canvas || bodyText.includes('Chart') || bodyText.includes('Loading');
    });
    expect(hasChartContent).toBe(true);
    
    // Should not crash
    const criticalErrors = errors.filter(error => 
      !error.includes('favicon') && 
      !error.includes('WebGPU') &&
      !error.includes('Warning')
    );
    expect(criticalErrors.length).toBe(0);
  });

  test('mouse interactions do not crash app', async ({ page }) => {
    await page.goto('/app');
    await page.waitForTimeout(3000);
    
    // Try basic mouse operations that shouldn't crash
    await page.mouse.move(400, 300);
    await page.mouse.wheel(0, -100);
    await page.mouse.move(500, 400);
    await page.mouse.wheel(0, 100);
    
    // Wait a bit to see if any crashes occur
    await page.waitForTimeout(1000);
    
    // App should still have content and not be in an error state
    const hasContent = await page.evaluate(() => {
      return document.body.children.length > 0;
    });
    expect(hasContent).toBe(true);
  });

  test('WASM module can be imported', async ({ page }) => {
    await page.goto('/app');
    
    // Check if WASM module loaded successfully
    const wasmLoaded = await page.evaluate(async () => {
      try {
        // Try to import the WASM module
        const wasmModule = await import('/pkg/tutorial1_window.js');
        return !!wasmModule;
      } catch {
        return false;
      }
    });
    
    // This is important - WASM should at least be importable
    expect(wasmLoaded).toBe(true);
  });

  test('basic React functionality works', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(2000);
    
    // Check that React rendered something
    const reactRendered = await page.evaluate(() => {
      // Look for React-specific patterns in the DOM
      const root = document.getElementById('root');
      return root && root.children.length > 0;
    });
    
    expect(reactRendered).toBe(true);
  });
});