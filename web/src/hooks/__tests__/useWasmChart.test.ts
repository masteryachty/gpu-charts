import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useWasmChart } from '../useWasmChart';
import { mockWasmInit, testUtils } from '../../test/mocks/wasm-mock';

// Mock the WASM module import
vi.mock('@pkg', () => ({
  default: mockWasmInit,
  ChartRenderer: testUtils.createMockChart(),
  initialize_logging: vi.fn(),
  get_version: () => '0.1.0-mock'
}));

describe('useWasmChart', () => {
  let mockCanvas: HTMLCanvasElement;

  beforeEach(() => {
    // Create a mock canvas element
    mockCanvas = document.createElement('canvas');
    testUtils.simulateCanvasResize(mockCanvas, 800, 600);
    
    // Mock getBoundingClientRect
    mockCanvas.getBoundingClientRect = vi.fn(() => ({
      x: 0,
      y: 0,
      width: 800,
      height: 600,
      top: 0,
      left: 0,
      bottom: 600,
      right: 800,
      toJSON: () => ({})
    }));

    // Mock getContext
    const mockContext = {
      fillStyle: '',
      fillRect: vi.fn(),
      strokeStyle: '',
      lineWidth: 1,
      beginPath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      stroke: vi.fn()
    };
    
    mockCanvas.getContext = vi.fn(() => mockContext);
    
    // Clear all mocks
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  it('should initialize with default state', () => {
    const { result } = renderHook(() => useWasmChart());

    expect(result.current.chartState.isInitialized).toBe(false);
    expect(result.current.chartState.isLoading).toBe(false);
    expect(result.current.chartState.error).toBeNull();
    expect(result.current.chartState.wasmModule).toBeNull();
    expect(result.current.chartState.chartRenderer).toBeNull();
    expect(result.current.tooltipData).toBeNull();
  });

  it('should initialize chart successfully', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    expect(result.current.chartState.isLoading).toBe(false);
    expect(result.current.chartState.error).toBeNull();
    expect(result.current.chartState.wasmModule).toBeTruthy();
    expect(result.current.chartState.chartRenderer).toBeTruthy();
  });

  it('should handle initialization errors', async () => {
    // Mock WASM initialization to fail
    vi.doMock('@pkg', () => ({
      default: () => Promise.reject(new Error('WASM initialization failed'))
    }));

    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.error).toBeTruthy();
    });

    expect(result.current.chartState.isInitialized).toBe(false);
    expect(result.current.chartState.isLoading).toBe(false);
    expect(result.current.chartState.error?.message).toContain('WASM initialization failed');
  });

  it('should set data correctly', async () => {
    const { result } = renderHook(() => useWasmChart());
    const mockData = testUtils.generateMockData(100);

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    act(() => {
      result.current.setData(mockData);
    });

    // Verify data was set (would need access to mock chart internals in real implementation)
    expect(result.current.chartState.chartRenderer).toBeTruthy();
  });

  it('should handle mouse events and generate tooltip data', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    act(() => {
      result.current.handleMouseMove(400, 300);
    });

    expect(result.current.tooltipData).toBeTruthy();
    expect(result.current.tooltipData?.x).toBe(400);
    expect(result.current.tooltipData?.y).toBe(300);
    expect(result.current.tooltipData?.price).toBeTypeOf('number');
    expect(result.current.tooltipData?.timestamp).toBeTypeOf('number');
  });

  it('should handle wheel events for zoom/pan', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    act(() => {
      result.current.handleWheel(10, 400, 300);
    });

    // Should not throw error and chart should remain initialized
    expect(result.current.chartState.isInitialized).toBe(true);
  });

  it('should update chart configuration', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    act(() => {
      result.current.setChartType('candlestick');
    });

    act(() => {
      result.current.setQualityPreset('high');
    });

    act(() => {
      result.current.setTheme('light');
    });

    // Should not throw errors
    expect(result.current.chartState.isInitialized).toBe(true);
  });

  it('should handle resize events', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    act(() => {
      result.current.handleResize(1200, 800);
    });

    // Should not throw error
    expect(result.current.chartState.isInitialized).toBe(true);
  });

  it('should handle view range updates', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    const startTime = Date.now() - 3600000; // 1 hour ago
    const endTime = Date.now();

    act(() => {
      result.current.setViewRange(startTime, endTime);
    });

    // Should not throw error
    expect(result.current.chartState.isInitialized).toBe(true);
  });

  it('should get performance stats', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    let stats;
    act(() => {
      stats = result.current.getPerformanceStats();
    });

    expect(stats).toBeTruthy();
    expect(stats.fps).toBeTypeOf('number');
    expect(stats.render_time).toBeTypeOf('number');
    expect(stats.data_points).toBeTypeOf('number');
  });

  it('should cleanup on disposal', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    act(() => {
      result.current.dispose();
    });

    expect(result.current.chartState.isInitialized).toBe(false);
    expect(result.current.chartState.chartRenderer).toBeNull();
    expect(result.current.tooltipData).toBeNull();
  });

  it('should handle keyboard events', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    act(() => {
      result.current.handleKeyDown('ArrowLeft');
    });

    act(() => {
      result.current.handleKeyUp('ArrowLeft');
    });

    // Should not throw errors
    expect(result.current.chartState.isInitialized).toBe(true);
  });

  it('should handle time range selection', async () => {
    const { result } = renderHook(() => useWasmChart());

    await act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    const startTime = Date.now() - 1800000; // 30 minutes ago
    const endTime = Date.now() - 900000; // 15 minutes ago

    act(() => {
      result.current.setTimeRangeSelection(startTime, endTime);
    });

    act(() => {
      result.current.clearTimeRangeSelection();
    });

    // Should not throw errors
    expect(result.current.chartState.isInitialized).toBe(true);
  });

  it('should prevent multiple initialization attempts', async () => {
    const { result } = renderHook(() => useWasmChart());

    // Start first initialization
    const promise1 = act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    // Start second initialization immediately
    const promise2 = act(async () => {
      await result.current.initializeChart(mockCanvas);
    });

    await Promise.all([promise1, promise2]);

    await waitFor(() => {
      expect(result.current.chartState.isInitialized).toBe(true);
    });

    // Should only initialize once
    expect(result.current.chartState.error).toBeNull();
  });
});