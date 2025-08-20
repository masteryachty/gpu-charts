#!/bin/bash

# Script to update visual regression baselines
# Usage: ./scripts/update-visual-baselines.sh [--ci-artifacts]

set -e

echo "Visual Regression Baseline Update Tool"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're updating from CI artifacts
if [ "$1" == "--ci-artifacts" ]; then
    echo -e "${YELLOW}Updating baselines from CI artifacts...${NC}"
    echo "Please download the 'cypress-screenshots' artifact from the failed GitHub Actions run"
    echo "Extract it to: web/cypress/screenshots/"
    read -p "Press Enter when ready to continue..."
    
    # Copy screenshots to baselines
    if [ -d "cypress/screenshots" ]; then
        echo -e "${GREEN}Copying screenshots to baselines...${NC}"
        
        # Create backup of current baselines
        if [ -d "cypress/fixtures/visual-baselines" ]; then
            echo "Creating backup of current baselines..."
            mv cypress/fixtures/visual-baselines cypress/fixtures/visual-baselines.backup.$(date +%Y%m%d_%H%M%S)
        fi
        
        # Create new baselines directory structure
        mkdir -p cypress/fixtures/visual-baselines
        
        # Copy all non-failed screenshots to baselines
        find cypress/screenshots -name "*.png" ! -name "*failed*" -exec cp {} cypress/fixtures/visual-baselines/ \;
        
        echo -e "${GREEN}Baselines updated successfully!${NC}"
    else
        echo -e "${RED}Error: No screenshots found in cypress/screenshots/${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}Generating new baselines locally...${NC}"
    
    # Run tests in baseline generation mode
    echo "Running Cypress tests to generate new baselines..."
    
    # Create temporary config for baseline generation
    cat > cypress.config.baseline.cjs << 'EOF'
const { defineConfig } = require('cypress');
const { configureVisualRegression } = require('cypress-visual-regression/dist/plugin');

module.exports = defineConfig({
  e2e: {
    baseUrl: 'http://localhost:3000',
    viewportWidth: 1280,
    viewportHeight: 720,
    video: false,
    screenshotsFolder: 'cypress/screenshots',
    screenshotOnRunFailure: true,
    defaultCommandTimeout: 10000,
    requestTimeout: 10000,
    responseTimeout: 10000,
    testIsolation: true,
    
    setupNodeEvents(on, config) {
      configureVisualRegression(on);
      return config;
    },
  },
  env: {
    visualRegressionType: 'base', // Generate baselines
    visualRegressionBaseDirectory: 'cypress/fixtures/visual-baselines',
    visualRegressionDiffDirectory: 'cypress/snapshots/diff',
    visualRegressionGenerateDiff: 'fail',
    visualRegressionFailSilently: false,
    visualRegressionFailureThreshold: 0,
    visualRegressionFailureThresholdType: 'percent',
  },
});
EOF

    # Check if servers are running
    echo "Checking if required servers are running..."
    if ! curl -s http://localhost:3000 > /dev/null; then
        echo -e "${RED}Error: React dev server is not running on port 3000${NC}"
        echo "Please run 'npm run dev:suite' in another terminal"
        exit 1
    fi
    
    # Run Cypress to generate baselines
    npx cypress run --config-file cypress.config.baseline.cjs --spec "cypress/e2e/visual-regression-*.cy.ts"
    
    # Clean up temporary config
    rm cypress.config.baseline.cjs
    
    echo -e "${GREEN}New baselines generated successfully!${NC}"
fi

echo ""
echo "Next steps:"
echo "1. Review the new baseline images in cypress/fixtures/visual-baselines/"
echo "2. Commit the updated baselines to git"
echo "3. Push to your branch to re-run CI tests"