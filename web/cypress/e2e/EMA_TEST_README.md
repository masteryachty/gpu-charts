# EMA Indicator Tests

This test suite verifies the functionality of Exponential Moving Average (EMA) indicators on the candlestick chart.

## Test Coverage

The test suite (`ema-indicators.cy.ts`) covers:

1. **EMA Toggle Visibility**: Verifies all 5 EMA indicators (9, 20, 50, 100, 200) appear in the metrics panel
2. **Individual EMA Control**: Tests toggling each EMA indicator on/off individually
3. **Multiple EMAs**: Tests rendering all EMAs together
4. **EMA Combinations**: Tests different combinations (short-term, medium-term, long-term)
5. **Zoom Persistence**: Ensures EMAs remain visible during zoom operations
6. **Color Verification**: Visual verification of EMA line colors
7. **Rapid Toggle Handling**: Tests system stability with rapid toggling
8. **Timeframe Changes**: Verifies EMAs adapt to candlestick timeframe changes

## Running the Tests

### Prerequisites
```bash
# Make sure the development environment is running
npm run dev:suite  # From project root

# Or run components individually:
npm run dev:wasm    # Build WASM
npm run dev:server  # Start data server
npm run dev         # Start React dev server
```

### Run EMA Tests

From the `web` directory:

```bash
# Run EMA tests in headless mode
npm run cy:ema

# Open Cypress GUI to run tests interactively
npm run cy:ema:open

# Run all Cypress tests
npm run test
```

### Test Output

- **Screenshots**: Saved to `cypress/screenshots/ema-indicators.cy.ts/`
- **Videos**: Disabled by default (can be enabled in `cypress.config.cjs`)
- **Test Results**: Displayed in terminal or Cypress GUI

## Expected Behavior

### EMA Configuration
- **EMA 9**: Light red color (RGB: 1.0, 0.4, 0.4)
- **EMA 20**: Orange color (RGB: 1.0, 0.6, 0.2)
- **EMA 50**: Yellow color (RGB: 1.0, 1.0, 0.4)
- **EMA 100**: Light green color (RGB: 0.4, 1.0, 0.4)
- **EMA 200**: Light blue color (RGB: 0.4, 0.6, 1.0)

### Default State
- All EMAs are **unchecked** by default (not visible)
- EMAs can be toggled via checkboxes in the metrics panel
- EMAs are calculated from candle close prices (time-based, not tick-based)

## Troubleshooting

### Common Issues

1. **WebGPU Not Available**
   - Ensure Chrome/Chromium browser with WebGPU support
   - Check browser flags are enabled (handled by Cypress config)

2. **Data Not Loading**
   - Verify data server is running (`npm run dev:server`)
   - Check API endpoint is accessible at `https://localhost:8443/api/`

3. **EMAs Not Rendering**
   - Check browser console for WebGPU errors
   - Verify WASM module is built (`npm run dev:wasm`)
   - Ensure candlestick preset is selected

4. **Test Timeouts**
   - Increase timeout in `cypress.config.cjs` if needed
   - Default timeout is 30 seconds for WebGPU initialization

## Visual Regression

Screenshots are taken at key points for visual regression testing:
- Individual EMA enabled states
- All EMAs together
- Different EMA combinations
- Zoom and interaction states

To update baseline screenshots:
```bash
npm run cy:ema -- --env visualRegressionType=base
```

## Development

To modify or add tests:
1. Edit `cypress/e2e/ema-indicators.cy.ts`
2. Add custom commands in `cypress/support/commands.ts`
3. Run tests to verify changes

## Related Files

- **Test File**: `cypress/e2e/ema-indicators.cy.ts`
- **Custom Commands**: `cypress/support/commands.ts`
- **Cypress Config**: `cypress.config.cjs`
- **EMA Implementation**: `crates/config-system/src/presets/candle_presets.rs`
- **Compute Engine**: `crates/renderer/src/compute_engine.rs`