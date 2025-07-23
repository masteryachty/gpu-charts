import { defineConfig, devices } from '@playwright/test';

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  testDir: './tests',
  /* Increase timeout for complex WASM/WebGPU tests */
  timeout: 30 * 1000,
  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: 'html',
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: 'http://localhost:3000',

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: 'on-first-retry',

    /* Take screenshot on failure */
    screenshot: 'only-on-failure',

    /* Record video on failure */
    video: 'retain-on-failure',
  },

  /* Configure projects for major browsers */
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // Enable WebGPU for testing with software fallback
        launchOptions: {
          args: [
            "--no-sandbox",
            "--disable-setuid-sandbox",
            "--disable-dev-shm-usage",
            "--disable-accelerated-2d-canvas",
            "--no-first-run",
            "--no-zygote",
            "--single-process", // Important for headless stability
            "--disable-gpu-sandbox",
            // WebGPU flags
            '--enable-unsafe-webgpu',
            '--enable-webgpu-developer-features',
            '--disable-web-security',
            '--ignore-certificate-errors',
            '--allow-running-insecure-content',
            // Graphics backend flags with fallbacks
            "--use-angle=swiftshader", // Software renderer fallback
            "--enable-features=VaapiVideoDecoder,WebGPU",
            "--disable-features=VizDisplayCompositor",
            // Memory and stability
            "--memory-pressure-off",
            "--max_old_space_size=4096",
            // Disable problematic features that can cause crashes
            "--disable-background-timer-throttling",
            "--disable-backgrounding-occluded-windows",
            "--disable-renderer-backgrounding",
            "--disable-features=TranslateUI",
            "--disable-component-extensions-with-background-pages",
            // Force software rendering for WebGL/WebGPU compatibility
            "--disable-gpu",
            "--disable-software-rasterizer"
          ]
        }
      },
    },

    {
      name: 'chromium-headless',
      use: {
        ...devices['Desktop Chrome'],
        // Headless mode with software rendering for CI/headless environments
        launchOptions: {
          headless: true,
          args: [
            "--no-sandbox",
            "--disable-setuid-sandbox",
            "--disable-dev-shm-usage",
            "--disable-gpu",
            "--disable-gpu-sandbox", 
            "--disable-software-rasterizer",
            "--disable-background-timer-throttling",
            "--disable-backgrounding-occluded-windows",
            "--disable-renderer-backgrounding",
            "--disable-web-security",
            "--ignore-certificate-errors",
            "--allow-running-insecure-content",
            "--no-first-run",
            "--no-zygote",
            "--single-process",
            "--memory-pressure-off",
            // Force canvas to use software rendering
            "--use-gl=swiftshader",
            "--use-angle=swiftshader",
            // Disable WebGPU for pure software fallback
            "--disable-features=WebGPU"
          ]
        }
      },
    },

    {
      name: 'firefox',
      use: {
        ...devices['Desktop Firefox'],
        // Firefox WebGPU is experimental
        launchOptions: {
          firefoxUserPrefs: {
            'dom.webgpu.enabled': true,
          }
        }
      },
    },

    {
      name: 'webkit',
      use: {
        ...devices['Desktop Safari'],
        // WebKit has limited WebGPU support
      },
    },

    /* Test against mobile viewports. */
    // {
    //   name: 'Mobile Chrome',
    //   use: { ...devices['Pixel 5'] },
    // },
    // {
    //   name: 'Mobile Safari',
    //   use: { ...devices['iPhone 12'] },
    // },

    /* Test against branded browsers. */
    // {
    //   name: 'Microsoft Edge',
    //   use: { ...devices['Desktop Edge'], channel: 'msedge' },
    // },
    // {
    //   name: 'Google Chrome',
    //   use: { ...devices['Desktop Chrome'], channel: 'chrome' },
    // },
  ],

  /* Run your local dev server before starting the tests */
  webServer: [
    {
      command: 'npm run dev',
      url: 'http://localhost:3000',
      reuseExistingServer: !process.env.CI,
      timeout: 120 * 1000, // 2 minutes for WASM to build
    },
    {
      command: 'node tests/test-server.js --port 8080 --http',
      url: 'http://localhost:8080/health',
      reuseExistingServer: !process.env.CI,
      timeout: 30 * 1000, // 30 seconds for test server
    }
  ],
});