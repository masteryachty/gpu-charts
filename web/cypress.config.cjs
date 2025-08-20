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
    defaultCommandTimeout: 10000,
    requestTimeout: 10000,
    responseTimeout: 10000,

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
    visualRegressionGenerateDiff: 'always', // Always generate diff images for CI
    visualRegressionFailSilently: false,
    visualRegressionFailureThreshold: 0.05, // Allow up to 5% difference for CI environments
    visualRegressionFailureThresholdType: 'percent',
  },
});