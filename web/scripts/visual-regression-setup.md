# Visual Regression Testing Setup

## Overview
The visual regression tests are now properly configured to compare screenshots against baseline images.

## Configuration Details

### cypress-visual-regression Plugin
- **Plugin**: `cypress-visual-regression` v5.3.0
- **Config File**: `cypress.config.cjs`
- **Support File**: `cypress/support/e2e.ts`

### Directory Structure
```
cypress/
├── fixtures/
│   └── visual-baselines/     # Baseline images for comparison
├── screenshots/               # New screenshots taken during tests
│   └── diff/                  # Diff images showing differences (created on failure)
```

### Environment Variables
- `visualRegressionType`: 
  - `'base'` - Generate new baseline images
  - `'regression'` - Compare against existing baselines (default)
- `visualRegressionBaseDirectory`: Where baseline images are stored
- `visualRegressionDiffDirectory`: Where diff images are saved
- `visualRegressionFailSilently`: Whether to fail tests on mismatch (false by default)

## Commands

### Generate New Baselines
```bash
# Generate all new baseline images (overwrites existing)
npm run cy:visual:update

# Or run with environment variable
npx cypress run --spec "cypress/e2e/visual-regression-*.cy.ts" --env visualRegressionType=base
```

### Run Visual Regression Tests
```bash
# Run all visual regression tests
npm run cy:visual

# Run specific test suites
npm run cy:visual:presets      # Preset tests only
npm run cy:visual:viewports    # Viewport tests only
npm run cy:visual:interactions # Interaction tests only
npm run cy:visual:metrics      # Metrics tests only
```

### How It Works

1. **Taking Screenshots**: Tests use `cy.matchImageSnapshot('name')` instead of `cy.screenshot()`
2. **Comparison**: The plugin automatically compares new screenshots with baselines
3. **Failure Threshold**: Set to 3% difference by default (configurable)
4. **Diff Images**: When tests fail, diff images are created showing the differences

### Test Failure Behavior

When a visual regression test fails:
1. A diff image is created in `cypress/screenshots/diff/`
2. The test fails with details about the percentage difference
3. You can review the diff image to see what changed

### Updating Baselines

When intentional UI changes are made:
1. Review the failing tests to confirm changes are expected
2. Run `npm run cy:visual:update` to update all baselines
3. Or manually copy the new screenshots from `cypress/screenshots/` to `cypress/fixtures/visual-baselines/`
4. Commit the updated baseline images

### Troubleshooting

**Tests not comparing properly:**
- Ensure `visualRegressionType` is set to `'regression'` (not `'base'`)
- Check that baseline images exist in `cypress/fixtures/visual-baselines/`
- Verify the image names match between tests and baselines

**False positives:**
- Adjust `failureThreshold` in `cypress/support/e2e.ts` if needed
- Consider using `customDiffConfig` for more control
- Ensure consistent test environment (viewport, timing, data)

**Missing baselines:**
- Run `npm run cy:visual:update` to generate initial baselines
- Commit baseline images to version control

## Best Practices

1. **Consistent Environment**: Always run tests with the same viewport and data
2. **Wait for Stability**: Use appropriate waits for animations/rendering
3. **Version Control**: Commit baseline images to track UI changes over time
4. **Review Diffs**: Always review diff images before updating baselines
5. **Selective Updates**: Update only specific baselines when needed rather than all