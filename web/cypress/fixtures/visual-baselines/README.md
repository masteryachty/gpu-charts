# Visual Regression Baseline Images

This directory contains the baseline screenshots used for visual regression testing with Cypress.

## Baseline Images

### Chart Presets
- `default-chart-view.png` - Default chart view with BTC-USD data
- `market-data-preset.png` - Market Data preset with bid/ask/trades/mid lines
- `candlestick-preset.png` - Candlestick preset with OHLC data
- `preset-market-data.png` - Market Data preset after switching
- `preset-candlestick.png` - Candlestick preset after switching

### Metrics Configurations
- `market-data-all-metrics.png` - All market data metrics enabled
- `market-data-bid-ask-only.png` - Only bid and ask lines visible
- `market-data-no-metrics.png` - Chart with no metrics displayed

## Updating Baselines

To update baseline images after intentional visual changes:

1. Run tests to generate new screenshots:
   ```bash
   npm run cy:visual
   ```

2. Review the new screenshots in `cypress/screenshots/`

3. Copy approved screenshots to this directory:
   ```bash
   cp cypress/screenshots/[test-file]/[screenshot-name].png cypress/fixtures/visual-baselines/
   ```

4. Commit the updated baselines with your changes

## CI/CD Integration

These baseline images are used in the GitHub Actions workflow to detect visual regressions in pull requests.