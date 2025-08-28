import '@testing-library/jest-dom';
import { expect, afterEach, beforeAll, afterAll, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import { setupServer } from 'msw/node';
import { handlers } from './mocks/handlers';

// Test environment configuration
(global as any).__TEST_MODE__ = true;
(global as any).__DISABLE_WEBGPU__ = true;
(global as any).__FORCE_SOFTWARE_RENDERING__ = true;
(global as any).__TEST_TIMEOUT_OVERRIDE__ = 1000;

// Mock WebGPU API for tests
global.GPU = class GPU {} as any;
global.GPUAdapter = class GPUAdapter {} as any;
global.GPUDevice = class GPUDevice {} as any;

// Mock WebAssembly
(global as any).WebAssembly = {
  instantiate: () => Promise.resolve({}),
  compile: () => Promise.resolve({}),
  instantiateStreaming: () => Promise.resolve({}),
  compileStreaming: () => Promise.resolve({})
};

// Enhanced ResizeObserver mock
global.ResizeObserver = class ResizeObserver {
  constructor(private callback: ResizeObserverCallback) {}
  
  observe(target: Element) {
    // Simulate observation with mock entry
    const mockEntry = {
      target,
      contentRect: { width: 800, height: 600, x: 0, y: 0, top: 0, left: 0, bottom: 600, right: 800 },
      borderBoxSize: [{ blockSize: 600, inlineSize: 800 }],
      contentBoxSize: [{ blockSize: 600, inlineSize: 800 }],
      devicePixelContentBoxSize: [{ blockSize: 600, inlineSize: 800 }]
    };
    setTimeout(() => this.callback([mockEntry as any], this as any), 0);
  }
  
  unobserve() {}
  disconnect() {}
} as any;

// Mock IntersectionObserver
global.IntersectionObserver = class IntersectionObserver {
  constructor() {}
  observe() {}
  unobserve() {}
  disconnect() {}
} as any;

// Mock requestAnimationFrame
(global as any).requestAnimationFrame = vi.fn((cb) => setTimeout(cb, 16));
(global as any).cancelAnimationFrame = vi.fn(clearTimeout);

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: vi.fn((key: string) => store[key] || null),
    setItem: vi.fn((key: string, value: string) => { store[key] = value.toString(); }),
    removeItem: vi.fn((key: string) => { delete store[key]; }),
    clear: vi.fn(() => { store = {}; }),
    get length() { return Object.keys(store).length; },
    key: vi.fn((index: number) => Object.keys(store)[index] || null),
  };
})();

Object.defineProperty(window, 'localStorage', { value: localStorageMock });

// Mock performance API
Object.defineProperty(window, 'performance', {
  value: {
    now: vi.fn(() => Date.now()),
    mark: vi.fn(),
    measure: vi.fn(),
    getEntriesByType: vi.fn(() => []),
    getEntriesByName: vi.fn(() => []),
  }
});

// Mock matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation(query => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// Setup MSW server
const server = setupServer(...handlers);

// Global test utilities
(global as any).testUtils = {
  waitForWasm: async () => new Promise(resolve => setTimeout(resolve, 100)),
  
  mockWebGPUSupport: (supported: boolean = false) => {
    if (supported) {
      Object.defineProperty(window.navigator, 'gpu', {
        value: {
          requestAdapter: vi.fn().mockResolvedValue({
            requestDevice: vi.fn().mockResolvedValue({})
          })
        },
        configurable: true,
      });
    } else {
      Object.defineProperty(window.navigator, 'gpu', {
        value: undefined,
        configurable: true,
      });
    }
  },
  
  simulateViewportResize: (width: number, height: number) => {
    Object.defineProperty(window, 'innerWidth', { writable: true, configurable: true, value: width });
    Object.defineProperty(window, 'innerHeight', { writable: true, configurable: true, value: height });
    window.dispatchEvent(new Event('resize'));
  }
};

const originalError = console.error;

beforeAll(() => {
  server.listen({ onUnhandledRequest: 'error' });
  
  // Suppress noisy console messages in tests
  console.error = (...args: any[]) => {
    if (
      typeof args[0] === 'string' &&
      (args[0].includes('Warning: ReactDOM.render') ||
       args[0].includes('Warning: useLayoutEffect') ||
       args[0].includes('Not implemented'))
    ) {
      return;
    }
    originalError.call(console, ...args);
  };
});

afterEach(() => {
  cleanup();
  server.resetHandlers();
  localStorageMock.clear();
  vi.clearAllMocks();
});

afterAll(() => {
  server.close();
  console.error = originalError;
});