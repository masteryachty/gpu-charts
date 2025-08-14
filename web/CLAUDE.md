# Web Frontend - CLAUDE.md

This file provides comprehensive guidance for working with the React frontend application, its WASM integration patterns, and modern frontend development practices.

## Purpose and Architecture

The web directory contains a high-performance React 18 application that serves as the frontend for a GPU-accelerated financial data visualization platform. It integrates WebAssembly modules built from Rust, provides real-time charting capabilities through WebGPU, and implements professional trading interface patterns.

### Core Responsibilities
- **WASM Integration**: Seamless bridging between React and Rust/WebAssembly modules
- **State Management**: Centralized application state with Zustand
- **Real-time Visualization**: WebGPU-powered chart rendering at 120fps
- **User Interface**: Professional trading dashboard with responsive controls
- **Error Handling**: Comprehensive error boundaries and recovery mechanisms

## Technology Stack

### Core Technologies
- **React 18.3.1**: Latest React with concurrent features and Suspense
- **TypeScript 5.6.2**: Strict type safety with advanced type features
- **Vite 6.3.5**: Lightning-fast build tool with HMR and WASM support
- **Tailwind CSS 3.4.13**: Utility-first styling with dark theme optimization
- **Zustand 4.4.7**: Lightweight state management without boilerplate

### Supporting Libraries
- **React Router DOM 6.26.1**: Client-side routing with future flags
- **Lucide React**: Modern icon library for UI components
- **clsx**: Conditional className utility
- **vite-plugin-wasm**: WebAssembly module loading
- **vite-plugin-top-level-await**: Async WASM initialization support

### Testing Infrastructure
- **Playwright 1.52.0**: End-to-end testing with WebGPU support
- **Multiple browser targets**: Chromium, Firefox, WebKit testing
- **Software rendering fallback**: Headless testing support

## Component Hierarchy and Organization

```
src/
├── App.tsx                         # Root application with routing and error boundaries
├── main.tsx                        # Entry point with BrowserRouter setup
│
├── pages/
│   ├── HomePage.tsx                # Landing page with feature showcase
│   └── TradingApp.tsx              # Main trading interface with chart integration
│
├── components/
│   ├── chart/
│   │   ├── WasmCanvas.tsx         # Core WASM chart integration component
│   │   └── ChartControls.tsx      # Chart control panel with presets
│   │
│   ├── layout/
│   │   ├── Header.tsx              # Top navigation with symbol search
│   │   ├── Sidebar.tsx             # Collapsible tools and indicators panel
│   │   └── StatusBar.tsx           # Bottom status information
│   │
│   ├── error/
│   │   ├── ErrorBoundary.tsx      # React error boundary with recovery
│   │   ├── ErrorNotificationCenter.tsx # Global error notifications
│   │   └── index.ts                # Error component exports
│   │
│   ├── PresetSection.tsx          # Preset selector with metric toggles
│   └── index.ts                    # Component barrel exports
│
├── hooks/
│   └── useWasmChart.ts            # WASM chart initialization and lifecycle
│
├── store/
│   └── useAppStore.ts             # Zustand store with subscriptions
│
├── config/
│   └── store-constants.ts         # Store configuration constants
│
├── errors/
│   ├── ErrorTypes.ts              # Error type definitions
│   ├── ErrorHandler.ts.disabled   # Advanced error handling (disabled)
│   └── index.ts                   # Error exports
│
├── types/
│   └── global.d.ts                # Global type declarations
│
├── styles/
│   └── globals.css                # Tailwind base and custom styles
│
├── utils/
│   └── performance-monitor.ts     # Performance monitoring utilities
│
└── wasm-trigger.ts                # HMR trigger for WASM rebuilds
```

## State Management with Zustand

### Store Architecture
The application uses Zustand for state management, providing a simple yet powerful state solution without React Context overhead.

#### Core Store Structure (`store/useAppStore.ts`)
```typescript
interface StoreState {
  // Chart configuration
  preset?: string;              // Active chart preset
  symbol?: string;              // Current trading symbol
  startTime: number;            // Chart start timestamp
  endTime: number;              // Chart end timestamp
}

interface AppStore extends StoreState {
  // Subscription management
  _subscriptions: Map<string, StoreSubscriptionCallbacks>;
  _lastState: StoreState | null;
  
  // Actions
  setCurrentSymbol: (symbol: string) => void;
  setPreset: (preset?: string) => void;
  setTimeRange: (startTime: number, endTime: number) => void;
  updateChartState: (updates: Partial<StoreState>) => void;
  resetToDefaults: () => void;
  
  // Subscription API
  subscribe: (id: string, callbacks: StoreSubscriptionCallbacks) => () => void;
  unsubscribe: (id: string) => void;
}
```

### Subscription Pattern
The store implements a custom subscription system for fine-grained change detection:
- **Symbol changes**: Track when trading symbol updates
- **Time range changes**: Monitor chart time window modifications
- **Preset changes**: Detect quality/metric preset switches
- **General changes**: Catch-all for any state mutations

### Store Usage Patterns
```typescript
// Direct state access
const { symbol, preset } = useAppStore();

// Action usage
const setCurrentSymbol = useAppStore(state => state.setCurrentSymbol);

// Subscription hooks
const chartSubscription = useChartSubscription({
  onSymbolChange: (newSymbol, oldSymbol) => { /* handle */ },
  onTimeRangeChange: (newRange, oldRange) => { /* handle */ }
});
```

## WASM Integration Patterns

### Chart Initialization Hook (`hooks/useWasmChart.ts`)
The `useWasmChart` hook manages the complete WASM lifecycle:

1. **Canvas Detection**: Waits for canvas element availability
2. **WASM Loading**: Dynamic import of WebAssembly module
3. **Chart Creation**: Instantiates Rust Chart class
4. **WebGPU Init**: Initializes GPU context and pipelines
5. **Render Loop**: Starts animation frame-based rendering
6. **Cleanup**: Proper disposal on unmount

### WasmCanvas Component (`components/chart/WasmCanvas.tsx`)
Central component for WASM chart rendering:

#### Key Features
- **Responsive Sizing**: Automatic canvas dimension updates
- **Event Bridging**: Mouse events forwarded to WASM
- **Loading States**: Visual feedback during initialization
- **Error Recovery**: Fallback for WebGPU unavailability
- **Test Mode Support**: Software rendering for testing

#### Event Handling
```typescript
// Mouse wheel for zoom
handleMouseWheel(event) → chart.handle_mouse_wheel(deltaY, x, y)

// Mouse move for crosshair
handleMouseMove(event) → chart.handle_mouse_move(x, y)

// Mouse click for selection
handleMouseDown/Up(event) → chart.handle_mouse_click(x, y, pressed)
```

### WASM Module Structure
The WASM module (`@pkg/wasm_bridge.js`) exposes:
- `Chart` class with WebGPU rendering
- Preset management methods
- Metric visibility controls
- Performance metrics access
- Time range updates

## Chart Component Architecture

### WasmCanvas Integration Flow
1. **Component Mount** → Canvas ref created
2. **WASM Init** → useWasmChart hook triggered
3. **Chart Ready** → onChartReady callback fired
4. **Store Sync** → URL params parsed to store
5. **Preset Applied** → Chart configured with preset
6. **Data Fetched** → Automatic data loading
7. **Render Loop** → Continuous GPU rendering

### ChartControls Component
Provides user interface for chart configuration:
- **Symbol Selection**: Dropdown for trading pairs
- **Preset Management**: Quality and metric presets
- **Time Range**: Quick time range buttons
- **Metric Toggles**: Individual chart layer visibility
- **Reset Controls**: Return to default settings

### PresetSection Component
Manages chart presets and metrics:
- Loads available presets from WASM
- Fetches metrics for active preset
- Toggles individual metric visibility
- Syncs preset state with store

## Error Handling and Boundaries

### ErrorBoundary Component
Comprehensive error catching with recovery:

#### Features
- **Automatic Recovery**: Exponential backoff retry logic
- **Error Reporting**: Integration with error tracking
- **User Feedback**: Clear error UI with actions
- **State Preservation**: Attempts to maintain app state
- **Manual Recovery**: User-triggered retry options

#### Error UI Components
- Error details display with stack trace
- Recovery status indicators
- Action buttons (retry, reset, report)
- Help text with troubleshooting steps

### Error Types
- **WASM Errors**: WebAssembly initialization failures
- **WebGPU Errors**: GPU context or rendering issues
- **Network Errors**: Data fetching failures
- **React Errors**: Component rendering exceptions

## Layout Components

### Header Component
Top navigation bar with:
- Logo and branding
- Symbol search input
- Watchlist dropdown
- Connection status indicator
- User menu with settings

### Sidebar Component
Collapsible side panel featuring:
- Tool selection icons
- Indicator panel
- Drawing tools
- Alert management
- Chart settings
- Theme controls

### StatusBar Component
Bottom information bar showing:
- Current symbol
- Connection status
- Performance metrics
- System messages

## Page Structure

### HomePage
Landing page with:
- Hero section with CTA
- Feature highlights
- Performance metrics showcase
- Navigation to trading app

### TradingApp
Main application interface:
- Header navigation
- Sidebar tools
- Central chart canvas
- Control panels
- Status information

#### URL Parameter Handling
Parses and applies URL parameters:
- `topic`: Trading symbol (e.g., BTC-USD)
- `start`: Start timestamp
- `end`: End timestamp
- `preset`: Initial chart preset

## Build and Development Configuration

### Vite Configuration (`vite.config.ts`)
```typescript
export default defineConfig({
  plugins: [
    react(),                    // React Fast Refresh
    wasm(),                     // WASM module support
    topLevelAwait()            // Async module loading
  ],
  server: {
    port: 3000,
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp'  // Required for SharedArrayBuffer
    }
  },
  resolve: {
    alias: {
      '@pkg': './pkg'          // WASM package alias
    }
  }
})
```

### TypeScript Configuration (`tsconfig.json`)
- **Target**: ES2020 for modern features
- **Strict Mode**: Full type safety enabled
- **Module Resolution**: Bundler mode for Vite
- **Path Aliases**: `@/` for src, `@pkg/` for WASM
- **Exclusions**: Disabled experimental features

## CSS and Styling Approach

### Tailwind Configuration
Custom theme with:
- **Dark Theme Colors**: Pure black backgrounds
- **Accent Colors**: High-contrast trading colors
- **Custom Fonts**: Inter for UI, JetBrains Mono for data
- **Animations**: Price movements and loading states

### Global Styles (`styles/globals.css`)
- Tailwind base/components/utilities
- Custom scrollbar styling
- Button component classes
- Input field styling
- Card containers
- Text gradients

### Styling Patterns
- **Utility-First**: Tailwind classes for most styling
- **Component Classes**: Reusable `.btn-primary`, `.input-primary`
- **Dark Mode First**: Optimized for dark theme
- **High Contrast**: Clear visual hierarchy

## Testing Setup

### Playwright Configuration
- **Browsers**: Chromium, Firefox, WebKit
- **WebGPU Support**: Software rendering fallback
- **Test Categories**: Basic, data, performance
- **Parallel Execution**: Full parallelization
- **Artifacts**: Screenshots, videos on failure

### Test Server Setup
- Development server on port 3000
- Mock data server on port 8080
- Automatic server startup
- Reuse existing servers in dev

### Test Categories
1. **Basic Tests**: Page loading, navigation
2. **WASM Tests**: WebAssembly initialization
3. **Data Tests**: Chart data visualization
4. **Performance Tests**: Memory and load time

## Performance Monitoring

### Built-in Monitoring (`utils/performance-monitor.ts`)
- Frame rate tracking
- Memory usage monitoring
- Load time measurement
- Render performance metrics

### WebGPU Performance
- 120fps target frame rate
- Sub-16ms render times
- GPU memory management
- Efficient buffer updates

## Integration with Backend

### API Configuration
Default API endpoint: `api.rednax.io`

Override via environment variable:
```bash
REACT_APP_API_BASE_URL=https://localhost:8443
```

### Data Flow
1. **Symbol Selection** → Store update
2. **Store Change** → WASM notification
3. **WASM Request** → Backend API call
4. **Binary Response** → GPU buffer update
5. **GPU Render** → Canvas display

## Development Workflow

### Quick Start
```bash
# Install dependencies
npm install

# Generate SSL certificates
npm run setup:ssl

# Start full development stack
npm run dev:suite
```

### Hot Module Replacement
- **React Components**: Instant HMR with state preservation
- **WASM Changes**: Auto-rebuild via dev-build.sh
- **Style Changes**: Immediate Tailwind updates
- **Type Changes**: TypeScript recompilation

### Development Scripts
- `npm run dev`: React dev server only
- `npm run dev:web`: WASM watch + React
- `npm run dev:suite`: Full stack development
- `npm run dev:wasm`: One-time WASM build
- `npm run dev:watch`: Auto WASM rebuilding

### Building for Production
```bash
npm run build
```
Creates optimized production bundle with:
- Minified JavaScript
- Optimized WASM module
- Tree-shaken dependencies
- Code splitting
- Source maps

## Best Practices

### Component Development
1. Use functional components with hooks
2. Implement proper TypeScript types
3. Follow single responsibility principle
4. Add error boundaries for robustness
5. Write Playwright tests for new features

### State Management
1. Keep store minimal and flat
2. Use subscriptions for targeted updates
3. Avoid unnecessary re-renders
4. Implement optimistic updates
5. Handle async operations properly

### WASM Integration
1. Check WebGPU availability
2. Implement loading states
3. Handle initialization failures
4. Clean up on unmount
5. Monitor performance metrics

### Performance Optimization
1. Use React.memo for expensive components
2. Implement virtualization for lists
3. Debounce user inputs
4. Optimize re-render triggers
5. Monitor bundle size

### Error Handling
1. Wrap components in error boundaries
2. Provide user-friendly error messages
3. Implement recovery mechanisms
4. Log errors for debugging
5. Test error scenarios

## Common Issues and Solutions

### WASM Loading Failures
- Check WebGPU browser support
- Verify WASM module built correctly
- Ensure correct CORS headers
- Check canvas element availability

### Performance Issues
- Profile with React DevTools
- Check for unnecessary re-renders
- Optimize Zustand subscriptions
- Monitor WebGPU memory usage

### Testing Challenges
- Use software rendering for CI
- Mock WebGPU unavailable scenarios
- Test with various viewport sizes
- Verify cross-browser compatibility

## Directory-Specific Notes

### Package Management
- Uses npm for dependency management
- Workspace-aware for monorepo structure
- Platform-specific optional dependencies

### Build Artifacts
- `pkg/`: WASM module output
- `dist/`: Production build output
- `playwright-report/`: Test results

### Configuration Files
- `vite.config.ts`: Build configuration
- `tsconfig.json`: TypeScript settings
- `tailwind.config.js`: Styling configuration
- `playwright.config.ts`: Test setup
- `eslint.config.js`: Linting rules

This frontend application represents a sophisticated, production-ready React application with cutting-edge WebAssembly integration, comprehensive testing, and professional trading interface design optimized for high-performance financial data visualization.