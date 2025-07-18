const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();
  
  // Navigate to the app
  await page.goto('http://localhost:3000/app');
  
  // Wait for page to load
  await page.waitForTimeout(5000);
  
  // Take a screenshot
  await page.screenshot({ path: 'candlestick-test.png', fullPage: true });
  
  // Check if controls exist
  const chartControls = await page.locator('[data-testid="chart-controls"]').isVisible();
  console.log('Chart controls visible:', chartControls);
  
  const lineButton = await page.locator('button:has-text("Line")').count();
  console.log('Line buttons found:', lineButton);
  
  const candlestickButton = await page.locator('button:has-text("Candlestick")').count();
  console.log('Candlestick buttons found:', candlestickButton);
  
  // Try clicking candlestick if found
  if (await page.locator('button:has-text("Candlestick")').first().isVisible()) {
    await page.locator('button:has-text("Candlestick")').first().click();
    await page.waitForTimeout(2000);
    
    // Check for timeframe selector
    const timeframeSelect = await page.locator('[data-testid="timeframe-select"]').isVisible();
    console.log('Timeframe selector visible:', timeframeSelect);
    
    // Take another screenshot
    await page.screenshot({ path: 'candlestick-test-after-click.png', fullPage: true });
  }
  
  // Keep browser open for manual inspection
  console.log('Browser will stay open for manual testing. Press Ctrl+C to close.');
  await page.waitForTimeout(300000); // 5 minutes
})();