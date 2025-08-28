/// <reference types="vitest" />
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    css: true,
    reporter: ['verbose', 'html'],
    outputFile: {
      html: './test-results/unit-test-report.html'
    },
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      reportsDirectory: './test-results/coverage',
      exclude: [
        'node_modules/**',
        'src/test/**',
        '**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
        '**/*{.,-}test.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
        '**/*{.,-}spec.{js,mjs,cjs,ts,mts,cts,jsx,tsx}',
        '**/cypress/**',
        'src/main.tsx',
        'pkg/**',
        'dist/**',
        'test-results/**',
      ],
      include: [
        'src/**/*.{js,jsx,ts,tsx}',
      ],
      thresholds: {
        global: {
          branches: 70,
          functions: 70,
          lines: 70,
          statements: 70
        }
      }
    },
    // Mock WebGPU and WASM for testing
    alias: {
      '@pkg': fileURLToPath(new URL('./src/test/mocks/wasm-mock.ts', import.meta.url))
    },
    // Increase timeout for complex component tests
    testTimeout: 10000,
    hookTimeout: 10000,
    // Pool options for performance
    pool: 'threads',
    poolOptions: {
      threads: {
        singleThread: true, // Helps with WebGPU mocking
      }
    }
  },
  resolve: {
    alias: {
      '@pkg': fileURLToPath(new URL('./src/test/mocks/wasm-mock.ts', import.meta.url)),
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  }
})