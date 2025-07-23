# React Store → Rust Integration System

## Overview

This comprehensive React Store → Rust Integration system provides a production-ready, type-safe bridge between React state management and WebAssembly-powered Rust components. The system enables real-time synchronization, autonomous data fetching, comprehensive error handling, and performance optimization.

## Architecture

### Core Components

1. **Store Contract Foundation** (`src/types/`)
   - Type-safe interfaces between React and Rust
   - Runtime validation and serialization
   - Advanced TypeScript integration

2. **WASM Bridge System** (`src/hooks/useWasmChart.ts`)
   - Seamless React-Rust communication
   - Method calls with error handling
   - Performance monitoring

3. **Smart State Change Detection** (`src/store/useAppStore.ts`)
   - Granular change detection algorithms
   - Optimized diff calculations
   - Debounced updates

4. **Autonomous Data Fetching** (`src/services/DataFetchingService.ts`)
   - Intelligent caching with LRU eviction
   - Background data fetching
   - Predictive prefetching

5. **Comprehensive Error Handling** (`src/errors/`)
   - Centralized error management
   - Automatic recovery strategies
   - User-friendly notifications

6. **Performance Optimization** (`src/performance/`)
   - Real-time performance monitoring
   - Automatic optimization triggers
   - Memory leak detection

## Key Features

### ✅ Type Safety
- Complete TypeScript integration with branded types
- Runtime type validation and guards
- Compile-time safety for all React-Rust interactions

### ✅ Intelligent Synchronization
- Automatic store state synchronization with debouncing
- Smart change detection to minimize unnecessary updates
- Consistent state across React and Rust components

### ✅ Autonomous Data Management
- Intelligent caching with configurable eviction policies
- Background data fetching and updates
- Predictive prefetching based on user patterns

### ✅ Robust Error Handling
- Comprehensive error categorization and reporting
- Automatic recovery strategies with fallbacks
- User-friendly error notifications

### ✅ Performance Optimization
- Real-time monitoring of FPS, memory, and network metrics
- Automatic performance optimizations
- Memory leak detection and cleanup

### ✅ Production Ready
- Comprehensive test coverage with Playwright
- Error boundaries and graceful degradation
- Performance benchmarks and monitoring

## Quick Start

### Installation

```bash
# Install dependencies
npm install

# Build WASM module
npm run dev:wasm

# Start development server
npm run dev:full
```

### Basic Usage

```typescript
import { useWasmChart } from './hooks/useWasmChart';
import { useAppStore } from './store/useAppStore';

function ChartComponent() {
  const [chartState, chartAPI] = useWasmChart({
    canvasId: 'my-chart',
    enableAutoSync: true,
    enableDataFetching: true,
    enablePerformanceMonitoring: true
  });

  const { currentSymbol, setCurrentSymbol } = useAppStore();

  return (
    <div>
      <canvas id="my-chart" />
      <select 
        value={currentSymbol} 
        onChange={e => setCurrentSymbol(e.target.value)}
      >
        <option value="BTC-USD">Bitcoin</option>
        <option value="ETH-USD">Ethereum</option>
      </select>
    </div>
  );
}
```

## Configuration

### Application Configuration

```typescript
import { TypedConfiguration } from './types/advanced-types';

const config: TypedConfiguration = {
  chart: {
    defaultTimeframe: '1h',
    maxDataPoints: 10000,
    refreshInterval: 1000
  },
  performance: {
    fpsThreshold: 30,
    memoryThreshold: 500 * 1024 * 1024,
    enableOptimizations: true
  },
  data: {
    cacheSize: 100,
    retryAttempts: 3,
    timeoutMs: 10000
  },
  errors: {
    enableReporting: true,
    maxErrorHistory: 1000,
    autoRecovery: true
  }
};
```

### Environment Variables

```bash
# Development
REACT_APP_API_BASE_URL=https://localhost:8443
REACT_APP_ENABLE_DEBUG=true
REACT_APP_PERFORMANCE_MONITORING=true

# Production
REACT_APP_API_BASE_URL=https://api.yourapp.com
REACT_APP_ENABLE_DEBUG=false
REACT_APP_PERFORMANCE_MONITORING=false
```

## API Reference

### Hooks

#### `useWasmChart(options)`

Advanced hook for React-Rust chart integration.

**Parameters:**
- `canvasId: string` - Canvas element ID for WebGPU
- `enableAutoSync?: boolean` - Automatic store synchronization
- `enableDataFetching?: boolean` - Autonomous data fetching
- `enablePerformanceMonitoring?: boolean` - Performance tracking
- `maxRetries?: number` - Error recovery attempts
- `debounceMs?: number` - State change debouncing

**Returns:** `[WasmChartState, WasmChartAPI]`

#### `useErrorHandler(options)`

Comprehensive error handling integration.

**Parameters:**
- `subscribeToCategories?: string[]` - Error categories to monitor
- `subscribeToSeverities?: string[]` - Error severities to monitor
- `onError?: (error: AppError) => void` - Error callback
- `onRecovery?: (errorCode: string) => void` - Recovery callback

**Returns:** `[ErrorState, ErrorHandlerAPI]`

#### `useAutonomousDataFetching(options)`

Intelligent data fetching with caching and prefetching.

**Parameters:**
- `enableAutoFetch?: boolean` - Automatic data fetching
- `enablePrefetch?: boolean` - Predictive prefetching
- `debounceMs?: number` - Request debouncing
- `dataColumns?: string[]` - Data columns to fetch

**Returns:** `[DataFetchingState, DataFetchingAPI]`

### Services

#### `DataFetchingService`

Core service for autonomous data management.

```typescript
const service = new DataFetchingService({
  prefetchEnabled: true,
  streamingEnabled: false,
  backgroundFetchEnabled: true,
  maxConcurrentRequests: 6,
  cacheExpiryMs: 5 * 60 * 1000
});

// Fetch data
const response = await service.fetchData({
  symbol: 'BTC-USD',
  startTime: startTimestamp,
  endTime: endTimestamp,
  timeframe: '1h',
  columns: ['time', 'best_bid', 'best_ask'],
  priority: 'normal',
  reason: 'user_action'
});
```

#### `ErrorHandler`

Centralized error management system.

```typescript
const errorHandler = new ErrorHandler({
  maxErrorHistory: 1000,
  enableConsoleLogging: true,
  enableLocalStorage: true,
  autoRecoveryEnabled: true
});

// Report errors
await errorHandler.handleWasmError(
  'WASM_INIT_FAILED',
  'Chart initialization failed',
  { method: 'initialize', recoverable: true }
);

// Register recovery strategies
errorHandler.registerRecoveryStrategy({
  errorCode: 'WASM_INIT_FAILED',
  maxAttempts: 3,
  delayMs: 2000,
  action: async () => {
    // Recovery logic
    return true;
  }
});
```

#### `PerformanceMonitor`

Real-time performance tracking and optimization.

```typescript
const monitor = new PerformanceMonitor();

// Start monitoring
monitor.startMonitoring(1000); // 1 second intervals

// Subscribe to metrics
monitor.subscribe((metrics) => {
  console.log('Performance:', {
    fps: metrics.fps,
    memory: metrics.totalMemoryUsage,
    health: metrics.systemHealth
  });
});

// Get performance recommendations
const recommendations = monitor.getRecommendations();
```

### Components

#### `ErrorBoundary`

Enhanced React error boundary with auto-recovery.

```typescript
<ErrorBoundary
  enableAutoRecovery={true}
  maxRetryAttempts={3}
  enableReporting={true}
  componentName="MyComponent"
>
  <MyComponent />
</ErrorBoundary>
```

#### `ErrorNotificationCenter`

User-facing error notification system.

```typescript
<ErrorNotificationCenter
  position="top-right"
  maxNotifications={5}
  autoHideTimeoutMs={8000}
  enableSounds={false}
  showDetailedInfo={false}
/>
```

#### `DataFetchingMonitor`

Real-time data fetching monitoring and control.

```typescript
<DataFetchingMonitor
  showDetailedInfo={true}
  enableManualControls={true}
  showActivity={true}
  compactMode={false}
/>
```

## Testing

### Running Tests

```bash
# All tests
npm test

# Integration tests
npm run test:data

# Performance tests
npm run test:performance

# Server tests
npm run test:server
```

### Test Structure

```
tests/
├── integration/
│   ├── react-rust-integration.spec.ts    # Core integration tests
│   └── data-scenarios.spec.ts            # Data handling tests
├── performance/
│   └── performance-benchmarks.spec.ts    # Performance benchmarks
└── helpers/
    ├── test-utils.ts                     # General test utilities
    ├── data-mocks.ts                     # Mock data helpers
    └── integration-test-utils.ts         # Integration-specific utils
```

### Example Test

```typescript
test('should sync store changes to WASM automatically', async ({ page }) => {
  await page.goto('/app?symbol=BTC-USD&debug=true');
  await IntegrationTestUtils.waitForSystemReady(page);
  
  // Change store state
  await page.selectOption('[data-testid="symbol-selector"]', 'ETH-USD');
  
  // Verify synchronization
  await expect(page.locator('[title="Synced"]')).toBeVisible({ timeout: 2000 });
  
  // Verify consistency
  await IntegrationTestUtils.verifySystemConsistency(page, {
    symbol: 'ETH-USD',
    timeframe: '1h'
  });
});
```

## Performance Optimization

### Monitoring

The system automatically monitors:
- **FPS**: Frame rate performance
- **Memory**: JavaScript and WASM memory usage
- **Network**: Latency and bandwidth
- **CPU**: Estimated CPU utilization

### Automatic Optimizations

- Memory cleanup when usage exceeds thresholds
- Rendering quality reduction during performance issues
- Request batching and deduplication
- Background task prioritization

### Manual Optimization

```typescript
// Trigger manual optimization
const appliedOptimizations = await performanceMonitor.optimizePerformance();
console.log('Applied optimizations:', appliedOptimizations);

// Register custom optimization
performanceMonitor.registerOptimization({
  id: 'custom-optimization',
  name: 'Custom Performance Fix',
  description: 'Custom optimization for specific scenarios',
  severity: 'medium',
  enabled: true,
  conditions: (metrics) => metrics.fps < 20,
  action: async () => {
    // Custom optimization logic
    return true;
  }
});
```

## Error Handling

### Error Categories

- **WASM**: WebAssembly initialization and method errors
- **Data**: Data fetching and processing errors
- **Store**: State synchronization errors
- **Network**: Connectivity and server errors
- **Performance**: Performance threshold violations
- **Validation**: Input validation failures

### Error Recovery

```typescript
// Automatic recovery strategies are registered for:
- WASM initialization failures (3 attempts with exponential backoff)
- Data fetch failures (5 attempts with retry delays)
- Store synchronization errors (3 attempts with 500ms delay)
- Network connectivity issues (automatic retry on reconnection)
```

### Error Notifications

Users receive appropriate notifications based on error severity:
- **Low**: Silent logging only
- **Medium**: Non-intrusive notifications
- **High**: Prominent notifications with actions
- **Critical**: Modal dialogs with recovery options

## Type Safety

### Advanced Types

```typescript
import { SymbolId, Timestamp, Price, Volume } from './types/advanced-types';

// Branded types for enhanced safety
const symbol: SymbolId = 'BTC-USD' as SymbolId;
const price: Price = 50000 as Price;
const timestamp: Timestamp = Date.now() as Timestamp;

// Type guards for runtime validation
if (isSymbolId(userInput)) {
  // userInput is now guaranteed to be a valid SymbolId
  processSymbol(userInput);
}
```

### Runtime Validation

```typescript
import { validateAndTransform, isChartConfig } from './types/type-guards';

// Validate and transform data
const config = validateAndTransform(
  userConfig,
  isChartConfig,
  'Invalid chart configuration'
);
```

## Deployment

### Development

```bash
# Start complete development stack
npm run dev:suite

# Individual services
npm run dev          # React dev server
npm run dev:server   # Data server
npm run dev:wasm     # WASM build
```

### Production

```bash
# Build all components
npm run build
npm run build:server
npm run build:logger

# Test production build
npm run preview
```

### Docker Deployment

```dockerfile
FROM node:18-alpine

WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

EXPOSE 3000
CMD ["npm", "start"]
```

## Contributing

### Development Setup

1. Clone the repository
2. Install dependencies: `npm install`
3. Set up SSL certificates: `npm run setup:ssl`
4. Start development: `npm run dev:suite`

### Code Standards

- TypeScript strict mode enabled
- ESLint with custom rules
- Prettier for code formatting
- 100% test coverage for critical paths

### Commit Guidelines

```bash
feat: add new feature
fix: bug fix
docs: documentation changes
style: formatting changes
refactor: code refactoring
test: test additions/changes
perf: performance improvements
```

## Troubleshooting

### Common Issues

**WASM initialization fails**
- Verify WebGPU support in browser
- Check canvas element exists
- Ensure WASM files are properly built

**Performance issues**
- Check memory usage in performance monitor
- Verify data size isn't too large
- Enable performance optimizations

**Data fetching errors**
- Verify server is running on correct port
- Check SSL certificates are valid
- Ensure API endpoints are accessible

**Type errors**
- Update TypeScript to latest version
- Verify all types are properly imported
- Check runtime validation matches types

### Debug Mode

Enable debug mode with URL parameter: `?debug=true`

This enables:
- Detailed console logging
- Performance overlays
- Error boundary details
- State change tracking

## License

MIT License - see LICENSE file for details.

## Support

For questions and support:
- Create an issue in the repository
- Check the troubleshooting guide
- Review the test examples
- Consult the API documentation