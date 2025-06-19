# Dual-Metric Rendering Test Plan

This document outlines the comprehensive testing strategy for the dual-metric rendering system implemented in PR #10.

## 📋 Test Overview

The dual-metric system introduces significant changes across both the Rust/WASM backend and React frontend. This test plan ensures all functionality works correctly and maintains high code quality.

## 🎯 Test Categories

### 1. **Backend Tests (Rust/WASM)**

#### `charting/tests/dual_metric_tests.rs`
- **HSV to RGB Color Generation**: Tests color conversion algorithm for metric differentiation
- **Color Uniqueness**: Ensures different metrics get visually distinct colors
- **Metric Color Assignment**: Validates known metrics get expected colors (blue for bid, red for ask)
- **URL Construction**: Tests dynamic API URL building with selected metrics
- **Fallback Handling**: Tests default behavior when no metrics specified

**Run with**: `wasm-pack test --node --release`

### 2. **Frontend Unit Tests (React/TypeScript)**

#### `web/tests/unit/dual-metric-store.spec.ts`
- **Metric Validation**: Tests selectedMetrics array validation against VALID_COLUMNS
- **Add/Remove Operations**: Tests addMetric and removeMetric store actions
- **Minimum Requirements**: Prevents removing all metrics (at least one required)
- **Change Detection**: Ensures metric changes trigger proper notifications
- **Serialization**: Tests JSON serialization for WASM bridge communication

#### `web/tests/unit/store-validation.spec.ts` (Enhanced)
- **Dual-Metric Support**: Extended existing validation to include selectedMetrics
- **Edge Cases**: Empty arrays, invalid metric names, duplicate metrics
- **API Integration**: Tests fetch parameter extraction includes selected metrics
- **Performance**: Validates large metric arrays don't degrade performance

**Run with**: `npm test -- tests/unit/`

### 3. **UI Integration Tests (Playwright)**

#### `web/tests/integration/chart-controls-metrics.spec.ts`
- **Control Visibility**: All metric toggle buttons display correctly
- **Selection Changes**: Clicking buttons updates selection state
- **Visual Feedback**: Selected/unselected states show appropriate styling
- **Count Display**: Metric count updates when selection changes
- **Accessibility**: Keyboard navigation and ARIA attributes work
- **Rapid Interaction**: Handles fast clicking without breaking

#### `web/tests/integration/dual-metric-ui.spec.ts`
- **Network Requests**: Metric changes trigger API calls with correct columns
- **State Persistence**: Metric selection maintained during other interactions
- **Performance**: Multiple metrics don't impact chart responsiveness
- **Error Resilience**: Graceful handling of API failures
- **Visual Rendering**: Different metrics display with distinct colors

**Run with**: `npm test -- tests/integration/chart-controls-metrics.spec.ts`

### 4. **WASM Bridge Integration Tests**

#### `web/tests/integration/dual-metric-wasm-bridge.spec.ts`
- **State Passing**: selectedMetrics correctly passed from React to WASM
- **Change Propagation**: Store updates trigger WASM chart updates
- **Error Handling**: Graceful handling of WASM busy states (no borrow panics)
- **Debouncing**: Rapid changes don't overwhelm system
- **Consistency**: Chart state remains consistent across interactions

**Run with**: `npm test -- tests/integration/dual-metric-wasm-bridge.spec.ts`

## 🚀 Running Tests

### Quick Test Suite
```bash
# Run all dual-metric specific tests
./scripts/test-dual-metric.sh
```

### Individual Test Categories
```bash
# Backend WASM tests
cd charting && wasm-pack test --node

# Frontend unit tests  
cd web && npm test -- tests/unit/dual-metric-store.spec.ts

# UI integration tests
cd web && npm test -- tests/integration/chart-controls-metrics.spec.ts

# Full integration tests
cd web && npm test -- tests/integration/dual-metric-ui.spec.ts
```

### Complete Test Suite
```bash
# All tests including existing ones
npm test
```

## 📊 Test Coverage Goals

### Backend Coverage
- **✅ 90%+** of new dual-metric code paths
- **✅ 100%** of critical data flow (metric selection → API request)
- **✅ 100%** of color generation algorithm
- **✅ Edge cases** for URL construction and fallbacks

### Frontend Coverage  
- **✅ 95%+** of store metric management functions
- **✅ 100%** of UI control interactions
- **✅ 90%+** of WASM bridge communication
- **✅ Error scenarios** and validation edge cases

### Integration Coverage
- **✅ All metric combinations** (1-4 metrics simultaneously)
- **✅ All user interactions** (click, keyboard, rapid changes)
- **✅ Cross-component state** consistency
- **✅ Performance edge cases** (many metrics, rapid changes)

## 🔍 Test Scenarios

### Core Functionality
1. **Single Metric Selection**: Select only best_bid → API request includes only time,best_bid
2. **Dual Metric Selection**: Select best_bid + best_ask → Chart shows two colored lines
3. **All Metrics Selection**: Select all 4 metrics → Chart shows 4 distinct colored lines
4. **Metric Toggle**: Deselect best_ask → Chart updates to hide red line

### UI Interactions
1. **Prevent Empty Selection**: Try to deselect last metric → Button disabled or selection preserved
2. **Visual Feedback**: Selected metrics show blue background, unselected show gray
3. **Count Display**: Metric count updates: "Data Metrics (2)" when 2 selected
4. **Rapid Clicking**: Fast toggle clicks don't break UI state

### Error Handling
1. **Borrow Conflicts**: Rapid state changes don't cause WASM panics
2. **API Failures**: Network errors don't break metric selection UI
3. **Invalid Data**: Malformed API responses handled gracefully
4. **Performance**: Many metrics don't cause frame rate drops

### Cross-Component Integration
1. **State Persistence**: Change symbol → metric selection preserved
2. **Timeframe Changes**: Change timeframe → new data fetched with same metrics
3. **Chart Resize**: Resize window → all metrics still rendered correctly
4. **Page Refresh**: Reload page → metric selection restored from URL/storage

## 🐛 Known Test Limitations

### Current Constraints
- **Visual Testing**: Limited screenshot-based testing for color verification
- **WebGPU Mocking**: Difficult to fully mock WebGPU rendering in tests
- **Async Timing**: Some race conditions in rapid state changes
- **Performance Measurement**: Limited ability to measure GPU performance

### Future Enhancements
- **Visual Regression Tests**: Screenshot comparison for color accuracy
- **GPU Performance Tests**: WebGPU memory and render timing tests
- **Load Testing**: Stress testing with large datasets and many metrics
- **Accessibility Testing**: Screen reader and keyboard navigation validation

## ✅ Test Success Criteria

### All Tests Must Pass
- **Zero test failures** in CI pipeline
- **No console errors** during test execution  
- **No memory leaks** in browser testing
- **No WASM panics** or borrow errors

### Performance Requirements
- **< 100ms** metric selection response time
- **< 2MB** additional memory usage with 4 metrics
- **60fps** maintained during metric changes
- **< 500ms** API response time with multiple metrics

### User Experience
- **Intuitive controls** pass accessibility guidelines
- **Visual clarity** between selected/unselected states
- **Responsive feedback** for all user interactions
- **Error recovery** from any failure state

## 📈 Metrics and Monitoring

### Test Execution Metrics
- **Test Runtime**: All tests complete in < 5 minutes
- **Flaky Test Rate**: < 1% test failure rate from timing issues
- **Coverage Reports**: Automated coverage reporting in CI
- **Performance Baselines**: Regression detection for performance tests

### Quality Gates
- **Branch Protection**: All tests must pass before merge
- **Code Review**: Test changes require review
- **Documentation**: All new test files documented
- **Maintenance**: Tests updated with feature changes

## 🔄 Continuous Integration

### CI Pipeline Integration
```yaml
# GitHub Actions integration
- name: Test Dual-Metric System
  run: |
    npm run test:dual-metric
    npm run test:coverage
    npm run test:performance
```

### Automated Checks
- **Test execution** on every PR
- **Coverage reporting** with diff analysis  
- **Performance regression** detection
- **Visual diff approval** for UI changes

This comprehensive test plan ensures the dual-metric rendering system is robust, performant, and maintainable while providing excellent user experience.