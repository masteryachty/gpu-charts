const { defineConfig } = require('cypress');

module.exports = defineConfig({
  e2e: {
    baseUrl: 'http://localhost:3000',
    viewportWidth: 1280,
    viewportHeight: 720,
    video: false,
    screenshotsFolder: 'cypress/screenshots',
    screenshotOnRunFailure: true,
    
    setupNodeEvents(on, config) {
      // Enable experimental features for better WebGPU support
      on('before:browser:launch', (browser, launchOptions) => {
        if (browser.family === 'chromium' && browser.name !== 'electron') {
          // Enable WebGPU flags
          launchOptions.args.push('--enable-unsafe-webgpu');
          launchOptions.args.push('--enable-features=Vulkan');
          launchOptions.args.push('--use-angle=vulkan');
          launchOptions.args.push('--enable-gpu-rasterization');
          launchOptions.args.push('--enable-zero-copy');
          
          // Disable security features that might interfere
          launchOptions.args.push('--disable-web-security');
          launchOptions.args.push('--disable-features=IsolateOrigins,site-per-process');
          
          console.log('Chrome launch options:', launchOptions.args);
        }
        return launchOptions;
      });
      
      return config;
    },
    
    // Timeouts for WebGPU/WASM initialization
    defaultCommandTimeout: 30000,
    requestTimeout: 30000,
    responseTimeout: 30000,
    
    // Test isolation
    testIsolation: true,
  },
});