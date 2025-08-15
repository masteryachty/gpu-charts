# Visual Regression Testing Documentation

## Overview

This visual regression testing suite provides pixel-perfect screenshot comparisons for the WebGPU-based financial charting application. It captures and compares screenshots of charts across different presets, time ranges, and viewport sizes to detect unintended visual changes.

## Architecture

### Test Structure
- **Test Suite**: `visual-regression.spec.ts` - Main test file with all visual regression scenarios
- **Utilities**: `helpers/visual-test-utils.ts` - Helper functions for WebGPU/WASM setup and screenshot management
- **Baselines**: `visual-baselines/` - Directory storing reference screenshots
- **Configuration**: Updated `playwright.config.ts` with visual testing optimizations

### Key Features
- ✅ WebGPU initialization handling
- ✅ WASM module loading synchronization
- ✅ Fixed timestamps for consistent data
- ✅ Multiple viewport testing
- ✅ Preset switching validation
- ✅ Interactive chart testing (zoom, pan)
- ✅ Pixel-perfect comparison with configurable thresholds
- ✅ CI/CD integration with GitHub Actions

## Running Visual Tests

### Prerequisites
1. Ensure the development environment is running:
   ```bash
   npm run dev:suite  # Starts WASM, server, and React app
   ```

2. Install Playwright browsers (if not already installed):
   ```bash
   cd web
   npx playwright install chromium
   ```

### Basic Commands

#### Run visual regression tests:
```bash
npm run test:visual
```

#### Update baseline screenshots (when changes are intentional):
```bash
npm run test:visual:update
```

#### Run tests with UI mode for debugging:
```bash
npm run test:visual:ui
```

#### Run tests in headed mode (see browser):
```bash
npm run test:visual:headed
```

#### Generate and view HTML report:
```bash
npm run test:visual:report
```

#### Run in Docker (for CI consistency):
```bash
npm run test:visual:docker
```

## Test Coverage

### Chart Presets
The suite automatically tests all available presets found in the application:
- Market Data preset (default)
- Any additional presets loaded from WASM module
- Each preset with different metric visibility combinations

### Test Scenarios

1. **Preset Testing**
   - Default view for each preset
   - All metrics visible
   - Selective metric visibility (e.g., bid/ask only)
   - Rapid preset switching

2. **Time Range Testing**
   - 1 hour data (ETH-USD)
   - 1 day data (BTC-USD)
   - 1 week data (BTC-USD)

3. **Viewport Testing**
   - Desktop (1920x1080)
   - Laptop (1366x768)
   - Tablet (768x1024)

4. **Interaction Testing**
   - Zoom in/out
   - Pan left/right
   - Reset view

5. **Symbol Testing**
   - BTC-USD
   - ETH-USD
   - ADA-USD

6. **Edge Cases**
   - Empty data handling
   - Error states
   - Loading states

7. **Accessibility Testing**
   - High contrast mode
   - Focus states
   - Keyboard navigation

## Updating Baselines

### When to Update
Update baselines when:
- Intentional UI changes are made
- Chart rendering improvements are implemented
- New features are added
- Color scheme or styling updates

### How to Update

#### Local Development:
```bash
# Review changes first
npm run test:visual:headed  # See what's different

# If changes are expected, update baselines
npm run test:visual:update

# Commit the updated baselines
git add web/tests/visual-baselines/
git commit -m "chore: update visual baselines for [feature]"
```

#### CI/CD:
1. Trigger the workflow manually with "Update baselines" option
2. Or use the GitHub Actions UI to run with `update_baselines: true`

## CI/CD Integration

### GitHub Actions Workflow
The `.github/workflows/visual-regression.yml` workflow:
1. Runs on every PR and push to main
2. Builds WASM module
3. Starts test server
4. Runs visual regression tests
5. Uploads artifacts on failure
6. Comments on PR with results

### Reviewing Failed Tests
When tests fail in CI:
1. Check the PR comment for failed test names
2. Download artifacts from Actions tab
3. Review diff images locally
4. Update baselines if changes are intentional

## Best Practices

### Writing New Visual Tests
1. **Use Fixed Data**: Always use predefined timestamps from `TEST_DATA_RANGES`
2. **Wait for Stability**: Use `waitForCanvasStable()` before screenshots
3. **Mask Dynamic Elements**: Hide timestamps, loading indicators
4. **Document Intent**: Add comments explaining what each test validates
5. **Group Related Tests**: Use `test.describe()` blocks

### Maintaining Tests
1. **Regular Reviews**: Review baselines quarterly for drift
2. **Clean Obsolete Tests**: Remove tests for deprecated features
3. **Monitor Performance**: Track test execution time
4. **Version Control**: Use Git LFS for large baseline collections

### Debugging Failed Tests

#### Local Debugging:
```bash
# Run specific test with debugging
npx playwright test visual-regression.spec.ts -g "Market Data preset" --debug

# Use UI mode for interactive debugging
npm run test:visual:ui

# Generate trace for analysis
npx playwright test --trace on
```

#### Common Issues and Solutions:

1. **WebGPU Not Available**
   - Ensure Chromium is used (not Firefox/WebKit)
   - Check browser flags in playwright.config.ts
   - Try software rendering flags

2. **Flaky Tests**
   - Increase wait times in `waitForChartReady()`
   - Add `waitForCanvasStable()` before screenshots
   - Check for race conditions in data loading

3. **Pixel Differences**
   - Verify font rendering consistency
   - Check device scale factor settings
   - Ensure animations are disabled

4. **WASM Loading Issues**
   - Increase timeout in `waitForChartReady()`
   - Check console for WASM errors
   - Verify pkg/ directory exists

## Configuration

### Playwright Config
Key settings in `playwright.config.ts`:
```typescript
{
  snapshotDir: './tests/visual-baselines',
  updateSnapshots: process.env.UPDATE_SNAPSHOTS === 'true' ? 'all' : 'missing',
  use: {
    ignoreHTTPSErrors: true,
    actionTimeout: 10000,
  }
}
```

### Visual Comparison Thresholds
Adjust in test files:
```typescript
expect(screenshot).toMatchSnapshot('name.png', {
  maxDiffPixels: 100,  // Allowed pixel differences
  threshold: 0.2,      // Per-pixel threshold (0-1)
});
```

## Troubleshooting

### Test Failures Checklist
- [ ] Are all services running? (WASM, server, React)
- [ ] Is WebGPU available in the browser?
- [ ] Are baselines up to date?
- [ ] Is the viewport size correct?
- [ ] Are timestamps fixed (not using Date.now)?
- [ ] Are dynamic elements masked?

### Performance Optimization
- Run tests in parallel when possible
- Use `--project=chromium-visual` for visual tests only
- Cache WASM builds between runs
- Use Docker for consistent environment

## Future Enhancements

### Planned Improvements
- [ ] Percy.io or Chromatic integration for cloud-based comparisons
- [ ] Automatic baseline updates via PR comments
- [ ] Visual diff reporting in PR checks
- [ ] Cross-browser visual testing (when WebGPU support improves)
- [ ] Performance regression detection
- [ ] A/B testing support for UI experiments

### Contributing
When adding new visual tests:
1. Follow existing patterns in `visual-regression.spec.ts`
2. Add helper functions to `visual-test-utils.ts` if needed
3. Document new test scenarios in this README
4. Ensure tests pass locally before pushing
5. Update baselines if introducing visual changes

## Support

For issues or questions:
1. Check this documentation first
2. Review test output and error messages
3. Check browser console for WebGPU/WASM errors
4. Consult Playwright documentation for advanced features
5. Open an issue with reproduction steps and screenshots