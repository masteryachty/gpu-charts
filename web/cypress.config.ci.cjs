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
    // Timeouts for WebGPU/WASM initialization
    defaultCommandTimeout: 15000,
    requestTimeout: 15000,
    responseTimeout: 15000,

    // Test isolation
    testIsolation: true,
    
    setupNodeEvents(on, config) {
      // Add visual regression plugin
      configureVisualRegression(on);
      return config;
    },
  },
  env: {
    visualRegressionType: 'regression', // 'base' to generate baselines, 'regression' to compare
    visualRegressionBaseDirectory: 'cypress/fixtures/visual-baselines',
    visualRegressionDiffDirectory: 'cypress/snapshots/diff',
    visualRegressionActualDirectory: 'cypress/snapshots/actual',
    visualRegressionGenerateDiff: 'always', // Always generate diff images in CI
    visualRegressionFailSilently: false,
    visualRegressionFailureThreshold: 0.1, // Allow up to 10% difference in CI (due to rendering differences)
    visualRegressionFailureThresholdType: 'percent',
    // CI-specific settings
    CI: true,
    updateSnapshots: false,
  },
});