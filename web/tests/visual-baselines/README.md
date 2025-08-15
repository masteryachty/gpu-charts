# Visual Regression Test Baselines

This directory contains the baseline screenshots for visual regression testing. These images are the "expected" results that new screenshots are compared against.

## Directory Structure

Baseline images are organized by test name and will be created automatically when you run tests for the first time or update baselines.

Example structure after running tests:
```
visual-baselines/
├── market-data-default-desktop.png
├── market-data-all-metrics-desktop.png
├── market-data-bid-ask-only-desktop.png
├── preset-market-data-desktop.png
├── market-data-range-hour-desktop.png
├── market-data-range-day-desktop.png
├── market-data-range-week-desktop.png
├── market-data-desktop.png
├── market-data-laptop.png
├── market-data-tablet.png
├── market-data-zoomed-in.png
├── market-data-panned-right.png
├── market-data-reset-view.png
├── market-data-btc-usd.png
├── market-data-eth-usd.png
├── market-data-ada-usd.png
├── controls-panel-market-data.png
├── controls-panel-dropdown-open.png
└── ...
```

## Managing Baselines

### Initial Setup
When running visual tests for the first time, baselines will be automatically created:
```bash
npm run test:visual
```

### Updating Baselines
When you make intentional UI changes, update the baselines:
```bash
npm run test:visual:update
```

### Reviewing Changes
1. Failed tests will generate diff images showing the differences
2. Review these in the test report: `npm run test:visual:report`
3. If changes are expected, update baselines
4. If changes are unexpected, fix the regression

## Version Control

- ✅ **DO** commit baseline PNG files to the repository
- ✅ **DO** use meaningful commit messages when updating baselines
- ❌ **DON'T** commit diff, actual, or expected images (these are gitignored)
- ❌ **DON'T** update baselines without reviewing the changes

## Best Practices

1. **Keep baselines up to date**: Update when making intentional UI changes
2. **Review before committing**: Always visually inspect baseline updates
3. **Document changes**: Include why baselines were updated in commit messages
4. **Use consistent environment**: Baselines should be generated in the same environment (preferably CI)
5. **Clean up old baselines**: Remove baselines for deleted tests

## Troubleshooting

### Baselines not found
If tests fail with "baseline not found":
1. Run `npm run test:visual:update` to create initial baselines
2. Commit the new baseline files

### Pixel differences on different machines
If tests pass locally but fail in CI:
1. Use Docker for consistent rendering: `npm run test:visual:docker`
2. Generate baselines in CI and download them
3. Consider using cloud-based visual testing services

### Large file sizes
If baseline files are too large:
1. Consider using Git LFS for baseline storage
2. Optimize PNG compression
3. Reduce test coverage to essential scenarios