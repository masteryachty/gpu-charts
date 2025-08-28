# GPU Charts - Refactoring & Improvement Task List

## Overview
This document contains a comprehensive analysis and task list for improving the GPU Charts web application. The tasks are organized by priority and impact, covering React frontend, WASM integration, GPU performance, UX/UI, testing, and Rust backend architecture.

## 🚨 **CRITICAL PERFORMANCE FIXES** (Immediate Impact)

### Rust/Server Performance
- [ ] **Fix fake zero-copy in server** - Server copies all data despite claiming zero-copy (server/src/data.rs:60)
- [ ] **Change opt-level from "z" to 3** - Currently optimizing for size not speed in Cargo.toml
- [ ] **Replace string cache keys with typed keys** - Avoid string allocations in hot paths

### GPU/WebGPU Performance  
- [x] **Implement buffer pooling** - Stop creating new GPU buffers every frame ✅ 
- [x] **Fix synchronous GPU readbacks** - Async ring buffer system implemented ✅ 
- [x] **Cache bind groups** - Buffer pool integration completed ✅
- [x] **Switch from LineList to LineStrip** - Already implemented ✅

### React/WASM Bridge
- [x] **Memoize all WasmCanvas event handlers** - Already implemented with useCallback ✅
- [x] **Fix animation frame memory leak** - Proper cleanup in all error paths ✅
- [x] **Implement render debouncing** - RequestAnimationFrame-based debouncing implemented ✅

## 🏗️ **ARCHITECTURE REFACTORING** (High Priority)

### Component Splitting
- [x] **Split ChartControls.tsx (446 lines)** into: ✅
  - SymbolDisplay.tsx - Current symbol with exchange info
  - SymbolSelector.tsx - Symbol selection dropdown  
  - ComparisonModeToggle.tsx - Exchange comparison mode
  - TimeRangeSelector.tsx - Time range preset buttons

- [x] **Split monolithic useAppStore** into domain stores: ✅
  - useChartStore (chart state) - Chart data and comparison mode
  - useUIStore (UI preferences) - Theme, layout, and settings
  - useMarketDataStore (market data) - Data loading and caching
  - useAppStoreNew (compatibility layer) - Drop-in replacement

- [ ] **Extract GPU types from shared-types** - Create dedicated gpu-types crate

### React Architecture
- [x] **Create ChartContext** - Eliminate prop drilling of chartInstance ✅
- [x] **Extract reusable custom hooks**: ✅
  - useURLParams() - URL parameter management
  - useDebounce() - Function and value debouncing (enhanced existing)
  - useKeyboardShortcuts() - Keyboard shortcut management
  - useResizeObserver() - Element resize detection with breakpoints

- [ ] **Implement proper error boundaries** - Add component-level error handling

## ♿ **ACCESSIBILITY & UX** (User Experience Critical)

### Accessibility Fixes
- [ ] **Add full keyboard navigation** - Tab order, focus management
- [ ] **Add ARIA labels and roles** - Chart canvas, buttons, dropdowns
- [ ] **Fix color-only indicators** - Add text/icon alternatives
- [ ] **Implement skip navigation links**

### Responsive Design
- [ ] **Make sidebar responsive** - Convert to bottom nav on mobile
- [ ] **Fix chart controls on mobile** - Stack vertically on small screens
- [ ] **Add touch gesture support** - Pinch zoom, pan for mobile

### UX Improvements
- [ ] **Add loading skeletons** - Show while WASM initializes
- [ ] **Implement proper tooltip UX** - Current right-click-hold is non-standard
- [ ] **Add first-time user tutorial** - Complex interface needs guidance
- [ ] **Fix silent failures** - Show user-friendly error messages

## 🧪 **TESTING INFRASTRUCTURE** (Quality Assurance)

### Unit Testing
- [ ] **Add tests for useWasmChart hook** - Critical untested code
- [ ] **Test useAppStore state management** - Complex logic untested
- [ ] **Add WASM bridge integration tests**
- [ ] **Test data transformation pipelines**

### E2E Testing
- [ ] **Setup Playwright for E2E tests**
- [ ] **Add critical user flow tests**:
  - Symbol search and selection
  - Chart interaction (zoom/pan)
  - Data loading and display
  - Preset application

### Performance Testing
- [ ] **Add GPU rendering benchmarks**
- [ ] **Create memory leak detection tests**
- [ ] **Add frame rate performance tests**

## ⚡ **PERFORMANCE OPTIMIZATIONS** (Medium Priority)

### Bundle Size
- [ ] **Implement code splitting** - Lazy load chart features
- [ ] **Tree-shake lucide-react icons** - Currently importing entire library
- [ ] **Add dynamic imports for heavy components**

### WASM Optimizations
- [ ] **Implement SharedArrayBuffer** - Zero-copy data transfer
- [ ] **Move WASM to Web Worker** - Offload from main thread
- [ ] **Add progressive WASM loading** - Split core vs features
- [ ] **Implement request batching** - Reduce JS-WASM boundary crossings

### GPU Optimizations
- [ ] **Optimize compute shader workgroups** - Adapt to GPU architecture
- [ ] **Implement command buffer batching** - Single submission per frame
- [ ] **Add push constants** - For frequently changing uniforms
- [ ] **Use warp-level primitives** - GPU vendor optimizations

## 🔧 **CODE QUALITY** (Maintainability)

### TypeScript Improvements
- [ ] **Generate TypeScript types from Rust** - Eliminate `any` types
- [ ] **Fix all TypeScript strict mode violations**
- [ ] **Add proper WASM module type definitions**

### Documentation
- [ ] **Add JSDoc comments** - Document complex functions
- [ ] **Create architecture decision records**
- [ ] **Add inline code examples**

### Code Cleanup
- [ ] **Standardize error handling** - Use consistent patterns
- [ ] **Remove code duplication** - Extract common utilities
- [ ] **Fix inconsistent spacing/styling** - Standardize Tailwind usage
- [ ] **Remove unused dependencies**

## 🚀 **FEATURE ENHANCEMENTS** (Lower Priority)

### Professional Trading Features
- [ ] **Add customizable workspace layouts**
- [ ] **Implement keyboard shortcuts system**
- [ ] **Add chart annotation tools**
- [ ] **Create market depth visualization**
- [ ] **Add order entry forms**

### Advanced Features
- [ ] **Implement workspace save/load**
- [ ] **Add multi-chart synchronization**
- [ ] **Create custom indicator system**
- [ ] **Add alert/notification system**

## 📊 **MONITORING & OBSERVABILITY**

- [ ] **Add performance monitoring** - Track render times, FPS
- [ ] **Implement error tracking** - Sentry or similar
- [ ] **Add usage analytics** - Understand user patterns
- [ ] **Create performance dashboard**

## 🔐 **SECURITY & RELIABILITY**

- [ ] **Add input validation** - Sanitize all user inputs
- [ ] **Implement rate limiting** - Prevent API abuse
- [ ] **Add CORS configuration** - Proper origin controls
- [ ] **Implement graceful degradation** - Fallback for WebGPU failure

## 📋 **Detailed Technical Issues**

### React Component Issues

#### ChartControls.tsx (web/src/components/chart/ChartControls.tsx)
- **Size**: 446 lines - needs splitting
- **Issues**:
  - Mixed responsibilities (exchange selection, time ranges, presets)
  - Complex conditional logic
  - No memoization
  - Inline event handlers

#### WasmCanvas.tsx (web/src/components/chart/WasmCanvas.tsx)
- **Size**: 363 lines
- **Issues**:
  - Event handlers recreated every render (lines 217-313)
  - Direct DOM manipulation
  - Missing cleanup in useEffect
  - No error boundaries

#### useAppStore.ts (web/src/store/useAppStore.ts)
- **Size**: 287 lines
- **Issues**:
  - Manual subscription management (error-prone)
  - Complex toggleExchange logic (lines 156-190)
  - Monolithic store design
  - Inefficient subscription triggers

### GPU/WebGPU Performance Issues

#### Buffer Management (crates/renderer/src/plot_renderer.rs)
- **Issue**: Creates new buffers per frame in `create_bind_group_for_metric()`
- **Impact**: 40-60% allocation overhead
- **Solution**: Implement buffer pooling

#### Compute Shaders (crates/renderer/src/shaders/min_max_first.wgsl)
- **Issue**: Fixed 256-thread workgroups
- **Impact**: Poor GPU occupancy
- **Solution**: Adaptive workgroup sizing

#### Synchronous Readbacks (crates/data-manager/src/compute_engine.rs)
- **Issue**: Blocking GPU pipeline with immediate buffer mapping
- **Impact**: 5-10ms stalls per frame
- **Solution**: Implement ring buffer for async readbacks

### WASM Bridge Issues

#### Memory Leaks (web/src/hooks/useWasmChart.ts)
- **Location**: Animation frame not cleaned on error
- **Impact**: Memory accumulation in long sessions
- **Solution**: Proper cleanup in all code paths

#### Type Safety (web/src/pages/TradingApp.tsx)
- **Issue**: `(chartInstance as any).apply_preset_and_symbols` (line 29)
- **Impact**: Runtime errors, poor IDE support
- **Solution**: Generate TypeScript definitions from Rust

### Rust Backend Issues

#### Server Zero-Copy Fake (server/src/data.rs)
- **Location**: Line 60 - `Vec::from(&mmap[start..end])`
- **Issue**: Copies data instead of zero-copy
- **Impact**: Defeats purpose of mmap
- **Solution**: Return slice references or use Bytes

#### Build Configuration (Cargo.toml)
- **Issue**: `opt-level = "z"` optimizes for size
- **Impact**: ~20-30% performance loss
- **Solution**: Change to `opt-level = 3`

## 🗓️ **Recommended Execution Timeline**

### **Phase 1 (Week 1-2): Critical Performance**
Focus on the first 10 critical performance fixes. These will have immediate, measurable impact.

### **Phase 2 (Week 3-4): Architecture & Accessibility**
Address architecture refactoring and accessibility issues (items 11-27).

### **Phase 3 (Week 5-6): Testing Foundation**
Establish comprehensive testing infrastructure (items 28-36).

### **Phase 4 (Week 7-8): Optimizations**
Implement performance optimizations (items 37-47).

### **Phase 5 (Ongoing): Features & Polish**
Add professional trading features and polish based on user feedback.

## 📈 **Expected Impact**

### Performance Improvements
- **30-40% reduction** in unnecessary re-renders
- **20-25% smaller** JavaScript bundle size
- **40-60% reduction** in GPU buffer allocation overhead
- **Sub-millisecond** data query response (after server fix)
- **Elimination** of 5-10ms GPU stalls per frame

### User Experience Improvements
- **Full accessibility** compliance (WCAG 2.1 AA)
- **Mobile-responsive** design
- **50% reduction** in user friction points
- **Professional-grade** trading interface

### Code Quality Improvements
- **90%+ test coverage** (from current ~10%)
- **Improved maintainability** through modular architecture
- **Type-safe** WASM bridge
- **Standardized** error handling

## 🛠️ **Tools & Resources Needed**

### Development Tools
- Vitest for testing
- Playwright for E2E tests
- Chrome DevTools Performance tab
- WebGPU profiling tools
- Rust flamegraph for profiling

### Libraries to Add
- `@testing-library/react` for component testing
- `msw` for API mocking
- `comlink` for Web Worker communication
- `wasm-bindgen-futures` for better async support

### Documentation Resources
- WCAG 2.1 guidelines
- React performance best practices
- WebGPU optimization guides
- Rust async patterns

## 📝 **Notes**

- All file paths are relative to project root
- Line numbers refer to current state of codebase
- Performance metrics are estimates based on analysis
- Priority levels consider both impact and effort

## 🎯 **Success Metrics**

- [ ] Time to Interactive (TTI) < 2 seconds
- [ ] Frame rate consistently 60 FPS
- [ ] Lighthouse performance score > 90
- [ ] Zero accessibility violations
- [ ] Test coverage > 90%
- [ ] Bundle size < 500KB (excluding WASM)
- [ ] Memory usage stable over 24-hour period
- [ ] Sub-millisecond data query latency

---

*Generated from comprehensive codebase analysis using specialized AI agents for React, WASM, GPU performance, UX/UI, testing, and Rust backend architecture.*