import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import { visualizer } from 'rollup-plugin-visualizer'
import { fileURLToPath, URL } from 'node:url'

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [
    react(),
    wasm(),
    topLevelAwait(),
    // Bundle analysis in analyze mode
    mode === 'analyze' && visualizer({
      filename: 'dist/bundle-analysis.html',
      open: true,
      gzipSize: true,
      brotliSize: true,
    })
  ].filter(Boolean),
  server: {
    port: 3000,
    host: true,
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp'
    }
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    target: 'esnext',
    rollupOptions: {
      output: {
        manualChunks: {
          // Vendor chunks
          vendor: ['react', 'react-dom', 'react-router-dom'],
          ui: ['lucide-react', 'clsx'],
          state: ['zustand'],
          // Chart chunk - keep WASM imports together
          chart: [
            './src/components/chart/WasmCanvas.tsx',
            './src/components/chart/ChartControls.tsx', 
            './src/components/chart/ChartLegend.tsx',
            './src/hooks/useWasmChart.ts'
          ]
        }
      }
    }
  },
  publicDir: 'public',
  optimizeDeps: {
    exclude: ['tutorial1_window']
  },
  resolve: {
    alias: {
      '@pkg': fileURLToPath(new URL('./pkg', import.meta.url))
    }
  }
}))