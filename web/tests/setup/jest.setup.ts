/**
 * Jest Setup File
 * 
 * Global setup for Jest unit tests including mocks, polyfills,
 * and custom matchers for the store contract testing.
 */

import '@testing-library/jest-dom';

// Mock WASM module
jest.mock('@pkg/tutorial1_window.js', () => ({
  default: jest.fn(() => Promise.resolve()),
  SimpleChart: jest.fn().mockImplementation(() => ({
    init: jest.fn(),
    is_initialized: jest.fn(() => true),
    handle_mouse_wheel: jest.fn(),
    handle_mouse_move: jest.fn(),
    handle_mouse_click: jest.fn(),
    render: jest.fn(() => Promise.resolve()),
  }))
}));

// Mock browser APIs
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: jest.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: jest.fn(), // Deprecated
    removeListener: jest.fn(), // Deprecated
    addEventListener: jest.fn(),
    removeEventListener: jest.fn(),
    dispatchEvent: jest.fn(),
  })),
});

// Mock ResizeObserver
global.ResizeObserver = jest.fn().mockImplementation(() => ({
  observe: jest.fn(),
  unobserve: jest.fn(),
  disconnect: jest.fn(),
}));

// Mock IntersectionObserver
global.IntersectionObserver = jest.fn().mockImplementation(() => ({
  observe: jest.fn(),
  unobserve: jest.fn(),
  disconnect: jest.fn(),
}));

// Mock performance.memory
Object.defineProperty(performance, 'memory', {
  writable: true,
  value: {
    usedJSHeapSize: 10000000,
    totalJSHeapSize: 20000000,
    jsHeapSizeLimit: 100000000,
  }
});

// Mock requestAnimationFrame
global.requestAnimationFrame = jest.fn((cb) => {
  setTimeout(cb, 16);
  return 1;
});

global.cancelAnimationFrame = jest.fn();

// Mock WebGPU
Object.defineProperty(navigator, 'gpu', {
  writable: true,
  value: {
    requestAdapter: jest.fn(() => Promise.resolve({
      requestDevice: jest.fn(() => Promise.resolve({
        queue: { submit: jest.fn() },
        createBuffer: jest.fn(),
        createTexture: jest.fn(),
        createBindGroup: jest.fn(),
      }))
    }))
  }
});

// Mock canvas context
HTMLCanvasElement.prototype.getContext = jest.fn((contextType) => {
  if (contextType === 'webgl2' || contextType === 'webgl') {
    return {
      getExtension: jest.fn(() => ({
        loseContext: jest.fn(),
      })),
      createShader: jest.fn(),
      shaderSource: jest.fn(),
      compileShader: jest.fn(),
      createProgram: jest.fn(),
      attachShader: jest.fn(),
      linkProgram: jest.fn(),
      useProgram: jest.fn(),
      createBuffer: jest.fn(),
      bindBuffer: jest.fn(),
      bufferData: jest.fn(),
      vertexAttribPointer: jest.fn(),
      enableVertexAttribArray: jest.fn(),
      drawArrays: jest.fn(),
      viewport: jest.fn(),
      clearColor: jest.fn(),
      clear: jest.fn(),
      // WebGL constants
      VERTEX_SHADER: 35633,
      FRAGMENT_SHADER: 35632,
      ARRAY_BUFFER: 34962,
      STATIC_DRAW: 35044,
      COLOR_BUFFER_BIT: 16384,
      TRIANGLES: 4,
    };
  }
  return null;
});

// Mock local storage
const localStorageMock = (() => {
  let store: Record<string, string> = {};

  return {
    getItem: jest.fn((key: string) => store[key] || null),
    setItem: jest.fn((key: string, value: string) => {
      store[key] = value.toString();
    }),
    removeItem: jest.fn((key: string) => {
      delete store[key];
    }),
    clear: jest.fn(() => {
      store = {};
    }),
    get length() {
      return Object.keys(store).length;
    },
    key: jest.fn((index: number) => {
      const keys = Object.keys(store);
      return keys[index] || null;
    })
  };
})();

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock
});

// Mock console methods for cleaner test output
const originalConsoleError = console.error;
const originalConsoleWarn = console.warn;
const originalConsoleLog = console.log;

beforeEach(() => {
  // Reset console mocks before each test
  console.error = jest.fn();
  console.warn = jest.fn();
  console.log = jest.fn();
});

afterEach(() => {
  // Clean up mocks after each test
  jest.clearAllMocks();
  
  // Restore console methods for debugging when needed
  if (process.env.DEBUG_TESTS) {
    console.error = originalConsoleError;
    console.warn = originalConsoleWarn;
    console.log = originalConsoleLog;
  }
});

afterAll(() => {
  // Restore original console methods
  console.error = originalConsoleError;
  console.warn = originalConsoleWarn;
  console.log = originalConsoleLog;
});

// Custom Jest matchers for store contract testing
expect.extend({
  toBeValidStoreState(received) {
    const isValid = 
      received &&
      typeof received === 'object' &&
      typeof received.currentSymbol === 'string' &&
      typeof received.chartConfig === 'object' &&
      typeof received.marketData === 'object' &&
      typeof received.isConnected === 'boolean';

    return {
      message: () => 
        isValid 
          ? `Expected ${received} not to be a valid store state`
          : `Expected ${received} to be a valid store state`,
      pass: isValid,
    };
  },

  toBeValidChartConfig(received) {
    const isValid =
      received &&
      typeof received === 'object' &&
      typeof received.symbol === 'string' &&
      typeof received.timeframe === 'string' &&
      typeof received.startTime === 'number' &&
      typeof received.endTime === 'number' &&
      Array.isArray(received.indicators);

    return {
      message: () =>
        isValid
          ? `Expected ${received} not to be a valid chart config`
          : `Expected ${received} to be a valid chart config`,
      pass: isValid,
    };
  },

  toHaveValidationErrors(received) {
    const hasErrors = 
      received &&
      typeof received === 'object' &&
      Array.isArray(received.errors) &&
      received.errors.length > 0;

    return {
      message: () =>
        hasErrors
          ? `Expected validation result not to have errors`
          : `Expected validation result to have errors`,
      pass: hasErrors,
    };
  },

  toBeWithinPerformanceBudget(received, budget) {
    const isWithinBudget = typeof received === 'number' && received <= budget;

    return {
      message: () =>
        isWithinBudget
          ? `Expected ${received}ms to exceed performance budget of ${budget}ms`
          : `Expected ${received}ms to be within performance budget of ${budget}ms`,
      pass: isWithinBudget,
    };
  },
});

// Global test utilities
global.testUtils = {
  createMockStoreState: () => ({
    currentSymbol: 'BTC-USD',
    chartConfig: {
      symbol: 'BTC-USD',
      timeframe: '1h',
      startTime: 1000000,
      endTime: 1003600,
      indicators: []
    },
    marketData: {},
    isConnected: true,
    user: undefined
  }),

  createMockChartConfig: () => ({
    symbol: 'BTC-USD',
    timeframe: '1h',
    startTime: 1000000,
    endTime: 1003600,
    indicators: ['RSI', 'MACD']
  }),

  waitForNextTick: () => new Promise(resolve => setTimeout(resolve, 0)),
  
  mockPerformanceNow: (value: number) => {
    jest.spyOn(performance, 'now').mockReturnValue(value);
  },

  restorePerformanceNow: () => {
    jest.restoreAllMocks();
  }
};

// Type declarations for custom matchers
declare global {
  namespace jest {
    interface Matchers<R> {
      toBeValidStoreState(): R;
      toBeValidChartConfig(): R;
      toHaveValidationErrors(): R;
      toBeWithinPerformanceBudget(budget: number): R;
    }
  }
  
  var testUtils: {
    createMockStoreState: () => any;
    createMockChartConfig: () => any;
    waitForNextTick: () => Promise<void>;
    mockPerformanceNow: (value: number) => void;
    restorePerformanceNow: () => void;
  };
}