# Web Directory - CLAUDE.md

This file provides specific guidance for working with the React/TypeScript frontend, build system, and comprehensive testing infrastructure of the graph visualization application.

## Overview

The web directory contains a modern React 18 application built with TypeScript, Vite, and Tailwind CSS. It provides a sophisticated frontend for real-time financial data visualization, featuring seamless WebAssembly integration, comprehensive testing with Playwright, and a professional trading interface design.

## Development Commands

### Core Development Workflow
```bash
# Complete development stack (recommended)
npm run dev:suite     # WASM watch + data server + React dev server

# Individual components
npm run dev             # React dev server only (port 3000)
npm run dev:full        # WASM watch + React dev server
npm run dev:wasm        # Build WASM for development
npm run dev:watch       # Auto-rebuild WASM with hot reload
npm run dev:server      # Run data server (port 8443)
```

### Build Commands
```bash
# Production build
npm run build           # Complete production build (WASM + React)

# Component builds
npm run build:wasm      # Production WASM build
npm run build:server    # Production server build

# Development builds
npm run dev:server:build  # Development server build
```

### Testing Commands
```bash
# Comprehensive testing
npm test                # Full Playwright test suite
npm run test:data       # Data visualization specific tests
npm run test:basic      # Basic functionality tests

# Server testing
npm run test:server     # Server unit and integration tests
npm run test:server:api # Live server API tests

# Test utilities
npm run test:report     # Open test results report
npm run test:debug      # Debug mode with browser UI
```

### Setup Commands
```bash
# Initial setup
npm install             # Install all dependencies
npm run setup:ssl       # Generate SSL certificates for HTTPS

# Code quality
npm run lint            # ESLint + TypeScript checking
npm run type-check      # TypeScript compilation check
```

## Architecture Overview

### Technology Stack
- **React 18.3.1**: Modern React with concurrent features
- **TypeScript 5.6.2**: Strict type checking and advanced type features
- **Vite 6.3.5**: Fast build tool with hot module replacement
- **Tailwind CSS 3.4.13**: Utility-first styling framework
- **Zustand 4.4.7**: Lightweight state management
- **React Router DOM 6.26.1**: Client-side routing
- **Playwright 1.52.0**: Comprehensive end-to-end testing

### Component Architecture
```
src/
├── App.tsx                    # Root application with routing
├── main.tsx                   # Application entry point
├── components/
│   ├── chart/
│   │   └── WasmCanvas.tsx     # Core WebAssembly chart integration
│   ├── layout/
│   │   ├── Header.tsx         # Navigation and status bar
│   │   ├── Sidebar.tsx        # Tool panels and controls
│   │   └── StatusBar.tsx      # Market data and metrics
│   └── ui/                    # Reusable UI components
├── pages/
│   ├── HomePage.tsx           # Landing page
│   └── TradingApp.tsx         # Main application interface
├── store/
│   └── useAppStore.ts         # Zustand state management
├── hooks/                     # Custom React hooks
├── types/
│   ├── index.ts               # Application type definitions
│   └── wasm.d.ts              # WebAssembly module declarations
└── styles/
    └── globals.css            # Global styles and Tailwind base
```

## WebAssembly Integration

### WASM Package Configuration
```typescript
// vite.config.ts
export default defineConfig({
  plugins: [
    react(),
    wasm(),                 // WebAssembly module support
    topLevelAwait()         // Async WASM loading support
  ],
  resolve: {
    alias: {
      '@pkg': fileURLToPath(new URL('./pkg', import.meta.url))  // WASM package alias
    }
  },
  server: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp'  // Required for WebGPU/SharedArrayBuffer
    }
  }
})
```

### WASM Integration Pattern (`WasmCanvas.tsx`)
```typescript
const WasmCanvas: React.FC<WasmCanvasProps> = ({ symbol, timeRange }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const chartRef = useRef<SimpleChart | null>(null);
  
  useEffect(() => {
    const initializeWasm = async () => {
      try {
        // Dynamic WASM module import
        const wasmModule = await import('@pkg/tutorial1_window.js');
        await wasmModule.default();
        
        // Chart initialization with error handling
        const chart = new wasmModule.SimpleChart();
        chart.init(canvasRef.current!.id);
        chartRef.current = chart;
        
        setIsLoaded(true);
      } catch (error) {
        console.error('WASM initialization failed:', error);
        setError(error as Error);
      }
    };
    
    initializeWasm();
  }, []);
  
  // Mouse event bridging
  const handleMouseWheel = useCallback((event: React.WheelEvent) => {
    if (chartRef.current?.is_initialized()) {
      chartRef.current.handle_mouse_wheel(
        event.deltaY,
        event.clientX,
        event.clientY
      );
    }
  }, []);
  
  return (
    <canvas
      ref={canvasRef}
      id="wasm-canvas"
      onWheel={handleMouseWheel}
      className="w-full h-full"
    />
  );
};
```

### WASM Type Definitions (`types/wasm.d.ts`)
```typescript
declare module '@pkg/tutorial1_window' {
  export default function init(input?: any): Promise<any>;
  
  export class SimpleChart {
    constructor();
    init(canvas_id: string): void;
    is_initialized(): boolean;
    handle_mouse_wheel(delta: number, x: number, y: number): void;
    handle_mouse_move(x: number, y: number): void;
    handle_mouse_click(x: number, y: number): void;
  }
}
```

## State Management Architecture

### Zustand Store Pattern (`store/useAppStore.ts`)
```typescript
interface AppState {
  // Market data state
  currentSymbol: string;
  timeRange: TimeRange;
  marketData: Record<string, MarketData>;
  
  // UI state
  isConnected: boolean;
  sidebarOpen: boolean;
  chartConfig: ChartConfig;
  
  // Performance metrics
  fps: number;
  latency: number;
}

interface AppActions {
  // Data actions
  setCurrentSymbol: (symbol: string) => void;
  setTimeRange: (range: TimeRange) => void;
  updateMarketData: (symbol: string, data: MarketData) => void;
  
  // UI actions
  toggleSidebar: () => void;
  setChartConfig: (config: ChartConfig) => void;
  
  // System actions
  setConnectionStatus: (connected: boolean) => void;
  updatePerformanceMetrics: (fps: number, latency: number) => void;
}

export const useAppStore = create<AppState & AppActions>((set, get) => ({
  // Initial state
  currentSymbol: 'BTC-USD',
  timeRange: { start: Date.now() - 3600000, end: Date.now() },
  marketData: {},
  isConnected: false,
  sidebarOpen: true,
  chartConfig: defaultChartConfig,
  fps: 0,
  latency: 0,
  
  // Actions with optimistic updates
  setCurrentSymbol: (symbol) => set({ currentSymbol: symbol }),
  setTimeRange: (range) => set({ timeRange: range }),
  updateMarketData: (symbol, data) => set((state) => ({
    marketData: { ...state.marketData, [symbol]: data }
  })),
  // ... other actions
}));
```

### State Management Benefits
- **No Providers**: Direct hook consumption without context wrapping
- **TypeScript Native**: Full type inference and compile-time checking
- **Performance**: Minimal re-renders with selector optimization
- **DevTools**: Easy debugging and state inspection
- **Immer Integration**: Immutable updates with draft API

## Component Design Patterns

### Layout Components

#### Header Component (`components/layout/Header.tsx`)
```typescript
const Header: React.FC = () => {
  const { currentSymbol, isConnected, setCurrentSymbol } = useAppStore();
  
  return (
    <header className="bg-dark-800 border-b border-dark-600 px-6 py-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-6">
          <Logo />
          <SymbolSearch 
            value={currentSymbol}
            onChange={setCurrentSymbol}
          />
        </div>
        
        <div className="flex items-center space-x-4">
          <ConnectionStatus connected={isConnected} />
          <UserMenu />
        </div>
      </div>
    </header>
  );
};
```

#### Sidebar Component (`components/layout/Sidebar.tsx`)
```typescript
const Sidebar: React.FC = () => {
  const { sidebarOpen, toggleSidebar, chartConfig, setChartConfig } = useAppStore();
  
  return (
    <motion.aside
      initial={false}
      animate={{ width: sidebarOpen ? 280 : 0 }}
      className="bg-dark-900 border-r border-dark-600 overflow-hidden"
    >
      <div className="p-4 space-y-6">
        <IndicatorPanel />
        <DrawingTools />
        <ChartSettings 
          config={chartConfig}
          onChange={setChartConfig}
        />
      </div>
    </motion.aside>
  );
};
```

### Chart Integration Component

#### Advanced Canvas Management
```typescript
const WasmCanvas: React.FC<WasmCanvasProps> = ({ symbol, timeRange }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const chartRef = useRef<SimpleChart | null>(null);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);
  
  // Responsive canvas sizing
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    
    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        canvas.width = width * devicePixelRatio;
        canvas.height = height * devicePixelRatio;
        canvas.style.width = `${width}px`;
        canvas.style.height = `${height}px`;
        
        // Notify WASM of size change
        if (chartRef.current?.is_initialized()) {
          chartRef.current.handle_resize(width, height);
        }
      }
    });
    
    resizeObserver.observe(canvas);
    resizeObserverRef.current = resizeObserver;
    
    return () => resizeObserver.disconnect();
  }, []);
  
  // Performance monitoring
  useEffect(() => {
    const interval = setInterval(() => {
      if (chartRef.current?.is_initialized()) {
        const metrics = chartRef.current.get_performance_metrics();
        updatePerformanceMetrics(metrics.fps, metrics.latency);
      }
    }, 1000);
    
    return () => clearInterval(interval);
  }, []);
};
```

## TypeScript Configuration

### Advanced TypeScript Setup (`tsconfig.json`)
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"],
      "@pkg/*": ["./pkg/*"]
    }
  }
}
```

### Type Safety Patterns
```typescript
// Market data types with strict validation
interface MarketData {
  symbol: string;
  timestamp: number;
  price: number;
  volume: number;
  bid: number;
  ask: number;
}

// Chart configuration with defaults
interface ChartConfig {
  theme: 'dark' | 'light';
  timeframe: '1m' | '5m' | '15m' | '1h' | '4h' | '1d';
  indicators: IndicatorConfig[];
  crosshair: boolean;
  volume: boolean;
}

// Zustand store typing
type AppStore = AppState & AppActions;
```

## Styling and Design System

### Tailwind Configuration (`tailwind.config.js`)
```javascript
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        // Professional trading interface colors
        dark: {
          900: '#0a0a0b',
          800: '#1a1a1b',
          700: '#2d2d30',
          600: '#3e3e42',
          500: '#6e6e73'
        },
        accent: {
          blue: '#007aff',
          green: '#34c759',
          red: '#ff3b30',
          orange: '#ff9500',
          purple: '#af52de'
        }
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace']
      },
      animation: {
        'price-up': 'priceUp 0.3s ease-out',
        'price-down': 'priceDown 0.3s ease-out',
        'pulse-subtle': 'pulseSubtle 2s infinite'
      }
    }
  },
  plugins: []
};
```

### Component Styling Patterns
```css
/* Global styles with CSS variables */
:root {
  --color-chart-background: theme('colors.dark.900');
  --color-chart-grid: theme('colors.dark.600');
  --color-chart-text: theme('colors.slate.300');
}

/* Trading-specific animations */
@keyframes priceUp {
  0% { background-color: transparent; }
  50% { background-color: theme('colors.accent.green / 20%'); }
  100% { background-color: transparent; }
}

@keyframes priceDown {
  0% { background-color: transparent; }
  50% { background-color: theme('colors.accent.red / 20%'); }
  100% { background-color: transparent; }
}

/* Component utilities */
.btn-primary {
  @apply bg-accent-blue text-white px-6 py-3 font-medium 
         transition-all duration-200 hover:bg-accent-blue/90
         focus:outline-none focus:ring-2 focus:ring-accent-blue/50;
}

.text-gradient {
  @apply bg-gradient-to-r from-accent-green to-accent-blue 
         bg-clip-text text-transparent;
}
```

## Testing Infrastructure

### Playwright Configuration (`playwright.config.ts`)
```typescript
export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  
  use: {
    baseURL: 'http://localhost:3000',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure'
  },
  
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] }
    },
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] }
    },
    {
      name: 'webkit',
      use: { ...devices['Desktop Safari'] }
    }
  ],
  
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:3000',
    reuseExistingServer: !process.env.CI
  }
});
```

### Test Utilities (`tests/helpers/`)

#### GraphTestUtils for WebGPU Testing
```typescript
export class GraphTestUtils {
  static async checkWebGPUSupport(page: Page): Promise<boolean> {
    return await page.evaluate(() => 'gpu' in navigator);
  }
  
  static async waitForWasmLoad(page: Page, timeout = 10000): Promise<void> {
    await page.waitForFunction(() => {
      const canvas = document.querySelector('#wasm-canvas');
      return canvas && canvas.getAttribute('data-initialized') === 'true';
    }, { timeout });
  }
  
  static async measureMemoryUsage(page: Page): Promise<MemoryInfo> {
    return await page.evaluate(async () => {
      if ('measureUserAgentSpecificMemory' in performance) {
        return await performance.measureUserAgentSpecificMemory();
      }
      return (performance as any).memory;
    });
  }
  
  static async triggerChartInteraction(page: Page, action: 'zoom' | 'pan', coordinates: { x: number, y: number }): Promise<void> {
    const canvas = page.locator('#wasm-canvas');
    
    if (action === 'zoom') {
      await canvas.hover();
      await page.mouse.wheel(0, -100); // Zoom in
    } else if (action === 'pan') {
      await canvas.click(coordinates);
      await page.mouse.move(coordinates.x + 100, coordinates.y);
    }
    
    // Wait for chart update
    await page.waitForTimeout(100);
  }
}
```

#### DataMockHelper for Test Data
```typescript
export class DataMockHelper {
  static generateMarketData(symbol: string, count: number): MarketData[] {
    const basePrice = Math.random() * 50000 + 20000; // $20k-$70k range
    const startTime = Date.now() - (count * 60 * 1000); // 1 minute intervals
    
    return Array.from({ length: count }, (_, i) => {
      const price = basePrice + (Math.random() - 0.5) * 1000;
      const volume = Math.random() * 100;
      
      return {
        symbol,
        timestamp: startTime + (i * 60 * 1000),
        price,
        volume,
        bid: price - Math.random() * 10,
        ask: price + Math.random() * 10
      };
    });
  }
  
  static async mockServerResponse(page: Page, data: MarketData[]): Promise<void> {
    await page.route('**/api/data**', async (route) => {
      // Convert to binary format matching server protocol
      const binaryData = this.convertToBinaryFormat(data);
      
      await route.fulfill({
        status: 200,
        contentType: 'application/octet-stream',
        body: Buffer.from(binaryData)
      });
    });
  }
}
```

### Test Categories

#### Basic Tests (`tests/basic.spec.ts`)
```typescript
test.describe('Application Loading', () => {
  test('should load homepage', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('h1')).toContainText('Graph Visualization');
  });
  
  test('should navigate to trading app', async ({ page }) => {
    await page.goto('/app');
    await expect(page.locator('#wasm-canvas')).toBeVisible();
  });
});
```

#### WebGPU and WASM Tests (`tests/app.spec.ts`)
```typescript
test.describe('WASM Integration', () => {
  test('should support WebGPU', async ({ page }) => {
    await page.goto('/app');
    
    const webgpuSupported = await GraphTestUtils.checkWebGPUSupport(page);
    expect(webgpuSupported).toBe(true);
  });
  
  test('should initialize WASM chart', async ({ page }) => {
    await page.goto('/app');
    await GraphTestUtils.waitForWasmLoad(page);
    
    const canvas = page.locator('#wasm-canvas');
    await expect(canvas).toHaveAttribute('data-initialized', 'true');
  });
});
```

#### Performance Tests (`tests/performance.spec.ts`)
```typescript
test.describe('Performance', () => {
  test('should maintain stable memory usage', async ({ page }) => {
    await page.goto('/app');
    await GraphTestUtils.waitForWasmLoad(page);
    
    const initialMemory = await GraphTestUtils.measureMemoryUsage(page);
    
    // Simulate heavy chart interactions
    for (let i = 0; i < 50; i++) {
      await GraphTestUtils.triggerChartInteraction(page, 'zoom', { x: 400, y: 300 });
      await page.waitForTimeout(50);
    }
    
    const finalMemory = await GraphTestUtils.measureMemoryUsage(page);
    const memoryGrowth = finalMemory.used - initialMemory.used;
    const growthPercentage = (memoryGrowth / initialMemory.used) * 100;
    
    expect(growthPercentage).toBeLessThan(200); // Max 200% memory growth
  });
  
  test('should load within performance budget', async ({ page }) => {
    const startTime = Date.now();
    await page.goto('/app');
    await GraphTestUtils.waitForWasmLoad(page);
    const loadTime = Date.now() - startTime;
    
    expect(loadTime).toBeLessThan(10000); // 10 second budget
  });
});
```

## Build System Architecture

### Vite Configuration Features
```typescript
export default defineConfig({
  plugins: [
    react(),
    wasm(),
    topLevelAwait(),
    
    // Development plugins
    ...(process.env.NODE_ENV === 'development' ? [
      inspect() // Bundle inspection
    ] : [])
  ],
  
  build: {
    target: 'es2020',
    rollupOptions: {
      output: {
        manualChunks: {
          'react-vendor': ['react', 'react-dom'],
          'chart-vendor': ['@pkg/tutorial1_window'],
          'ui-vendor': ['lucide-react', 'framer-motion']
        }
      }
    }
  },
  
  optimizeDeps: {
    exclude: ['@pkg/tutorial1_window'] // Don't pre-bundle WASM
  }
});
```

### Hot Reload System
The development workflow includes sophisticated hot reload:

1. **Rust File Changes**: `scripts/dev-build.sh` watches for changes
2. **WASM Rebuild**: Automatic wasm-pack build on Rust changes
3. **Trigger File**: `wasm-trigger.ts` touched to trigger Vite reload
4. **React HMR**: Components reload with preserved state where possible

### Package Scripts Ecosystem
```json
{
  "scripts": {
    "dev": "vite",
    "dev:full": "concurrently \"npm run dev:watch\" \"npm run dev\"",
    "dev:suite": "concurrently \"npm run dev:watch\" \"npm run dev:server\" \"npm run dev\"",
    "dev:wasm": "cd .. && wasm-pack build --target web --out-dir web/pkg",
    "dev:watch": "../scripts/dev-build.sh",
    "dev:server": "cd ../server && cargo run --target x86_64-unknown-linux-gnu",
    
    "build": "npm run build:wasm && tsc && vite build",
    "build:wasm": "cd .. && wasm-pack build --release --target web --out-dir web/pkg",
    "build:server": "cd ../server && cargo build --release --target x86_64-unknown-linux-gnu",
    
    "test": "playwright test",
    "test:data": "playwright test tests/data-scenarios tests/data-visualization tests/simple-data-tests",
    "test:basic": "playwright test tests/basic tests/app",
    "test:server": "cd ../server && cargo test --target x86_64-unknown-linux-gnu",
    "test:server:api": "cd ../server && ./test_api.sh",
    
    "lint": "eslint . --ext ts,tsx --report-unused-disable-directives --max-warnings 0",
    "type-check": "tsc --noEmit",
    "setup:ssl": "../scripts/setup-ssl.sh"
  }
}
```

## Development Workflow

### Complete Development Environment
```bash
# Start complete development stack
npm run dev:suite

# This runs concurrently:
# 1. ../scripts/dev-build.sh    # WASM file watcher and rebuilder
# 2. ../server cargo run         # Data server on port 8443
# 3. vite                        # React dev server on port 3000
```

### Hot Reload Chain
1. **Edit Rust file** in `/src`
2. **File watcher** detects change
3. **WASM rebuild** via wasm-pack
4. **Trigger file** (`wasm-trigger.ts`) touched
5. **Vite HMR** reloads affected components
6. **Browser updates** without full page reload

### Testing Workflow
```bash
# Development testing
npm run test:basic      # Quick smoke tests
npm run test:data       # Data visualization tests

# Full testing pipeline
npm run test:server     # Backend tests
npm run test           # Full E2E suite
npm run test:report     # View test results
```

## Production Deployment

### Build Optimization
```bash
npm run build
```

This creates an optimized production build with:
- **Code Splitting**: Vendor chunks for efficient caching
- **Tree Shaking**: Unused code elimination
- **WASM Optimization**: Release builds with size optimization
- **Asset Optimization**: Image and font optimization
- **Source Maps**: Production debugging support

### Performance Monitoring
The application includes built-in performance monitoring:
- **FPS Tracking**: Real-time frame rate monitoring
- **Memory Usage**: WebAssembly memory tracking
- **Load Time Metrics**: Initial load and WASM initialization timing
- **Error Boundaries**: Comprehensive error catching and reporting

## Common Development Tasks

### Adding New React Components
1. Create component in appropriate directory (`components/`, `pages/`)
2. Follow naming convention: PascalCase for files and components
3. Include proper TypeScript typing
4. Add to component index file for clean imports
5. Write Playwright tests for new functionality

### Integrating New WASM Features
1. Add new methods to WASM bridge (`lib_react.rs`)
2. Update TypeScript declarations (`types/wasm.d.ts`)
3. Implement React component integration
4. Add event handling and state management
5. Write integration tests

### Styling New Components
1. Use Tailwind utility classes for layout and basic styling
2. Add custom CSS only when necessary
3. Follow dark-first design principles
4. Use CSS variables for dynamic theming
5. Ensure responsive design across breakpoints

### Debugging Issues
```typescript
// WASM debugging
console.log('WASM initialized:', chart?.is_initialized());

// State debugging with Zustand devtools
const useAppStore = create<AppStore>()(
  devtools(
    (set, get) => ({
      // ... store implementation
    }),
    { name: 'app-store' }
  )
);

// Performance debugging
const metrics = await performance.measureUserAgentSpecificMemory();
console.log('Memory usage:', metrics);
```

This React frontend represents a production-ready, high-performance application with exceptional developer experience, comprehensive testing, and seamless WebAssembly integration for real-time financial data visualization.