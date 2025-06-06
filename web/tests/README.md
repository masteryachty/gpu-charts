# E2E Testing with Playwright

This directory contains end-to-end tests for the Graph Visualization application using Playwright.

## Test Structure

### Core Test Files
- `app.spec.ts` - Main application functionality tests
- `performance.spec.ts` - Performance and memory leak detection tests
- `helpers/test-utils.ts` - Utility functions and test helpers

### Test Categories

#### Functionality Tests (`app.spec.ts`)
- **Application Loading**: WASM module loading, WebGPU initialization
- **Data Visualization**: Chart rendering with various data sources
- **User Interactions**: Zoom, pan, cursor tracking
- **Error Handling**: Network failures, invalid data, browser compatibility
- **Memory Management**: Sustained interaction testing

#### Performance Tests (`performance.spec.ts`)
- **Load Time**: Initial chart rendering performance
- **Large Datasets**: Handling of high-volume data
- **Memory Leaks**: Long-running interaction testing
- **Responsiveness**: Interaction response time measurement
- **Network Conditions**: Slow network simulation
- **Viewport Adaptation**: Multi-resolution testing

#### Browser Compatibility
- **Chromium**: Full WebGPU support testing
- **Firefox**: Experimental WebGPU testing
- **WebKit**: Fallback behavior testing

## Running Tests

### Local Development
```bash
# Run all tests
npm run test

# Run tests with browser UI
npm run test:headed

# Interactive test runner
npm run test:ui

# Debug mode (step through tests)
npm run test:debug

# View test reports
npm run test:report
```

### Test-Specific Commands
```bash
# Run only functionality tests
npx playwright test app.spec.ts

# Run only performance tests
npx playwright test performance.spec.ts

# Run specific browser
npx playwright test --project=chromium

# Run in specific browser with UI
npx playwright test --project=firefox --headed
```

## Test Configuration

### WebGPU Testing
The tests are configured to:
- Enable WebGPU flags in Chromium (`--enable-unsafe-webgpu`)
- Enable experimental WebGPU in Firefox (`dom.webgpu.enabled`)
- Test fallback behavior when WebGPU is unavailable

### Network Testing
- Slow network simulation (50kb/s download, 500ms latency)
- Network failure simulation (blocked requests)
- Data mocking for consistent test conditions

### Performance Budgets
- Initial load: < 10 seconds
- Large datasets (10k points): < 15 seconds
- Memory growth: < 200% during normal use
- Interaction response: < 100ms average, < 500ms max

## Test Data

### Time Ranges
- **Valid Range**: `start=1745322750&end=1745691150`
- **Invalid Range**: `start=9999999999&end=9999999990`

### Topics
- `BTC-usd` - Bitcoin price data
- `ETH-usd` - Ethereum price data  
- `sensor_data` - Sensor telemetry data

### Generated Data
The test utils can generate synthetic time series data:
```typescript
TestData.generateTimeSeriesData(1000, startTimestamp)
```

## CI/CD Integration

Tests run automatically on:
- Push to `main` or `develop` branches
- Pull requests to `main`

### GitHub Actions Workflow
- Builds WASM module
- Installs Playwright browsers
- Runs full test suite
- Uploads test reports and screenshots
- Matrix testing across browsers

## Debugging Tests

### Screenshots
Tests automatically capture screenshots on failure. For debugging:
```typescript
await utils.takeDebugScreenshot('custom-debug-name');
```

### Console Monitoring
```typescript
const errors = await utils.getConsoleErrors();
```

### Memory Profiling
```typescript
const memory = await utils.checkMemoryUsage();
console.log(`Memory usage: ${memory.used / 1024 / 1024}MB`);
```

### Video Recording
Tests record video on failure. Enable for all tests:
```typescript
// In playwright.config.ts
use: {
  video: 'on', // Always record
}
```

## Common Issues

### System Dependencies
If browser installation fails, install system dependencies:
```bash
npx playwright install-deps
```

### WASM Build Failures
Ensure Rust and wasm-pack are installed:
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

### WebGPU Unavailable
Some browsers/environments don't support WebGPU. Tests should:
- Detect WebGPU availability
- Test fallback behavior
- Skip WebGPU-specific tests when unavailable

### Timing Issues
If tests are flaky due to timing:
- Increase timeouts in `playwright.config.ts`
- Use `waitForFunction()` instead of fixed timeouts
- Wait for specific conditions rather than arbitrary delays

## Adding New Tests

### Test Structure
```typescript
test.describe('Feature Name', () => {
  let utils: GraphTestUtils;

  test.beforeEach(async ({ page }) => {
    utils = new GraphTestUtils(page);
  });

  test('should do something specific', async ({ page }) => {
    await utils.navigateToApp();
    await utils.waitForChartRender();
    
    // Test implementation
    
    await expect(page.locator('canvas')).toBeVisible();
  });
});
```

### Best Practices
1. Use test utilities for common operations
2. Wait for specific conditions, not arbitrary timeouts
3. Test error conditions and edge cases
4. Include performance considerations
5. Mock external dependencies when possible
6. Use descriptive test names and assertions