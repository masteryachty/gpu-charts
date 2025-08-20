# Visual Regression Testing Guide

## Overview
Visual regression tests ensure that the UI appearance remains consistent across code changes. These tests capture screenshots and compare them against baseline images.

## Configuration

### Local Development
- **Config File**: `cypress.config.cjs`
- **Threshold**: 2% difference allowed
- **Diff Generation**: Only on failure

### CI Environment
- **Config File**: `cypress.config.ci.cjs`
- **Threshold**: 10% difference allowed (accounts for rendering differences in CI)
- **Diff Generation**: Always (for debugging)

## Running Tests

### Local Testing
```bash
# Run all visual regression tests
npm run cy:visual

# Run specific test suites
npm run cy:visual:presets      # Preset switching tests
npm run cy:visual:viewports    # Different viewport tests
npm run cy:visual:interactions # User interaction tests
npm run cy:visual:metrics      # Metrics display tests
```

### Updating Baselines
```bash
# Generate new baseline images locally
npm run cy:visual:update

# Or use the update script (Windows)
cd web
scripts\update-visual-baselines.bat

# Or use the update script (Linux/Mac)
cd web
./scripts/update-visual-baselines.sh
```

## Handling CI Failures

When visual regression tests fail in GitHub Actions:

### 1. Review the Failure
- Check the GitHub Actions log for which tests failed
- Look for the percentage difference (e.g., "Threshold limit of '0.1' exceeded: '0.15'")

### 2. Download Artifacts
GitHub Actions uploads three artifact sets:
- **cypress-screenshots**: All test screenshots including failures
- **visual-regression-diffs**: Difference images showing what changed
- **visual-regression-baselines**: Current baseline images for comparison

To download:
1. Go to the failed GitHub Actions run
2. Scroll to the bottom "Artifacts" section
3. Download the relevant artifacts

### 3. Review the Differences

Extract the artifacts and review:
- Failed screenshots will have "(failed)" in the filename
- Diff images show the visual differences highlighted
- Compare with baseline images to understand what changed

### 4. Update Baselines (if changes are intentional)

If the visual changes are intentional:

#### Option A: Update from CI Artifacts
```bash
# Download cypress-screenshots artifact from GitHub
# Extract to web/cypress/screenshots/
cd web
scripts\update-visual-baselines.bat --ci-artifacts
```

#### Option B: Generate New Baselines Locally
```bash
# Start the development environment
npm run dev:suite

# In another terminal, generate new baselines
cd web
npm run cy:visual:update
```

### 5. Commit and Push
```bash
git add web/cypress/fixtures/visual-baselines/
git commit -m "Update visual regression baselines"
git push
```

## Common Issues and Solutions

### Issue: Tests fail with small percentage differences
**Cause**: Minor rendering differences between environments
**Solution**: The CI config allows 10% difference threshold

### Issue: Cannot generate diff images locally
**Cause**: Missing diff directory
**Solution**: Ensure `web/cypress/snapshots/diff/` directory exists

### Issue: Tests timeout waiting for chart
**Cause**: WebGPU initialization takes longer in CI
**Solution**: CI config has increased timeouts (15 seconds)

### Issue: Different results locally vs CI
**Cause**: Different Chrome versions or GPU rendering
**Solution**: Use the CI-specific threshold (10%) to account for differences

## Best Practices

1. **Review Changes Carefully**: Always review visual differences before updating baselines
2. **Document Changes**: When updating baselines, explain why in the commit message
3. **Test Locally First**: Run visual tests locally before pushing
4. **Keep Baselines Updated**: Update baselines when intentional UI changes are made
5. **Use Appropriate Thresholds**: CI uses higher threshold (10%) due to rendering differences

## Troubleshooting Commands

```bash
# Clear all screenshots and snapshots
rm -rf web/cypress/screenshots
rm -rf web/cypress/snapshots

# Run tests with specific config
npx cypress run --config-file cypress.config.ci.cjs

# Generate baselines for a specific test
npx cypress run --spec "cypress/e2e/visual-regression-viewports.cy.ts" --env visualRegressionType=base

# Run tests in headed mode for debugging
npx cypress open --e2e --browser chrome
```

## Directory Structure

```
web/
├── cypress/
│   ├── fixtures/
│   │   └── visual-baselines/     # Baseline images
│   │       └── cypress/e2e/      # Organized by test file
│   ├── screenshots/              # Test screenshots (gitignored)
│   ├── snapshots/                # Visual regression snapshots
│   │   ├── actual/              # Current screenshots
│   │   └── diff/                # Difference images
│   └── e2e/
│       └── visual-regression-*.cy.ts  # Test files
├── cypress.config.cjs            # Local config
├── cypress.config.ci.cjs         # CI config
└── scripts/
    ├── update-visual-baselines.sh   # Linux/Mac update script
    └── update-visual-baselines.bat  # Windows update script
```