import { useCallback, useEffect, useRef, useState } from 'react';
import { useAppStore } from '../store/useAppStore';
import type { ChartSystem } from '@pkg/gpu_charts_wasm';

/**
 * Configuration constants for the WASM chart hook
 */
const WASM_CHART_CONSTANTS = {
  DEFAULT_DEBOUNCE_MS: 100,
  INITIALIZATION_DELAY_MS: 50,
  WASM_LOAD_TIMEOUT_MS: 10000,
} as const;

/**
 * New WASM Chart Integration Hook using gpu-charts-wasm
 * 
 * Provides React-Rust bridge with the new architecture
 */
export interface UseWasmChartOptions {
  /** Canvas ID for WebGPU surface attachment */
  canvasId: string;
  
  /** API base URL for data fetching */
  apiBaseUrl?: string;
  
  /** Enable automatic store state synchronization */
  enableAutoSync?: boolean;
  
  /** Debounce delay for state changes (ms) */
  debounceMs?: number;
}

export interface WasmChartState {
  /** Chart instance from WASM */
  chart: ChartSystem | null;
  
  /** Initialization state */
  isInitialized: boolean;
  isLoading: boolean;
  
  /** Error handling */
  error: string | null;
  
  /** Performance metrics */
  fps: number;
  frameTime: number;
}

export interface WasmChartAPI {
  /** Manual chart operations */
  initialize: () => Promise<boolean>;
  updateChart: (chartType?: string, symbol?: string, startTime?: number, endTime?: number) => Promise<boolean>;
  render: () => Promise<boolean>;
  resize: (width: number, height: number) => void;
  
  /** Configuration */
  updateConfig: (configJson: string) => Promise<boolean>;
  getConfig: () => string;
  
  /** Performance */
  getStats: () => string;
  
  /** Cleanup */
  destroy: () => void;
}

export function useWasmChart(options: UseWasmChartOptions): [WasmChartState, WasmChartAPI] {
  const {
    canvasId,
    apiBaseUrl = 'https://api.rednax.io',
    enableAutoSync = true,
    debounceMs = WASM_CHART_CONSTANTS.DEFAULT_DEBOUNCE_MS,
  } = options;

  // Get specific store state values
  const storeSymbol = useAppStore(state => state.currentSymbol);
  const storeTimeframe = useAppStore(state => state.chartConfig.timeframe);
  const storeChartType = useAppStore(state => state.chartConfig.chartType);
  const storeStartTime = useAppStore(state => state.chartConfig.startTime);
  const storeEndTime = useAppStore(state => state.chartConfig.endTime);
  
  // Chart state management
  const [chartState, setChartState] = useState<WasmChartState>({
    chart: null,
    isInitialized: false,
    isLoading: false,
    error: null,
    fps: 0,
    frameTime: 0,
  });

  // Refs for cleanup and performance
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();
  const mountedRef = useRef(true);
  const animationFrameRef = useRef<number>();

  /**
   * Initialize the WASM chart instance
   */
  const initialize = useCallback(async (): Promise<boolean> => {
    if (!mountedRef.current) return false;

    setChartState(prev => ({
      ...prev,
      isLoading: true,
      error: null,
    }));

    try {
      console.log(`[useWasmChart] Initializing chart for canvas: ${canvasId}`);

      // Wait for canvas to be available
      let canvas: HTMLElement | null = null;
      let retries = 0;
      const maxCanvasRetries = 10;
      
      while (!canvas && retries < maxCanvasRetries) {
        canvas = document.getElementById(canvasId);
        if (!canvas) {
          console.log(`[useWasmChart] Canvas not found, waiting... (attempt ${retries + 1}/${maxCanvasRetries})`);
          await new Promise(resolve => setTimeout(resolve, 100));
          retries++;
        }
      }
      
      if (!canvas) {
        throw new Error(`Canvas with ID "${canvasId}" not found after ${maxCanvasRetries} attempts`);
      }

      // Dynamic WASM module import
      console.log('[useWasmChart] Loading WASM module...');
      const wasmModule = await import('@pkg/gpu_charts_wasm.js');
      console.log('[useWasmChart] WASM module imported, initializing...');
      await wasmModule.default();
      console.log('[useWasmChart] WASM module initialized');

      if (!mountedRef.current) {
        console.log('[useWasmChart] Component unmounted during WASM init');
        return false;
      }

      // Create ChartSystem instance 
      console.log('[useWasmChart] Creating ChartSystem instance...');
      const chart = await new wasmModule.ChartSystem(canvasId, apiBaseUrl);
      console.log('[useWasmChart] ChartSystem instance created');

      if (!mountedRef.current) return false;

      console.log('[useWasmChart] Chart initialized successfully');

      setChartState(prev => ({
        ...prev,
        chart,
        isInitialized: true,
        isLoading: false,
        error: null,
      }));

      // Trigger initial state sync if enabled
      if (enableAutoSync) {
        setTimeout(() => {
          updateChart();
        }, WASM_CHART_CONSTANTS.INITIALIZATION_DELAY_MS);
      }

      return true;
    } catch (error) {
      console.error('[useWasmChart] Initialization failed:', error);
      
      if (mountedRef.current) {
        setChartState(prev => ({
          ...prev,
          isLoading: false,
          error: `Initialization failed: ${error}`,
        }));
      }
      
      return false;
    }
  }, [canvasId, apiBaseUrl, enableAutoSync]);

  /**
   * Update chart with new data
   */
  const updateChart = useCallback(async (
    chartType?: string,
    symbol?: string,
    startTime?: number,
    endTime?: number
  ): Promise<boolean> => {
    if (!chartState.chart || !chartState.isInitialized) {
      console.warn('[useWasmChart] Cannot update chart - not initialized');
      return false;
    }

    try {
      const currentChartType = chartType || storeChartType || 'line';
      const currentSymbol = symbol || storeSymbol || 'BTC-USD';
      const currentStartTime = startTime || storeStartTime || Date.now() - 3600000; // 1 hour ago
      const currentEndTime = endTime || storeEndTime || Date.now();

      console.log('[useWasmChart] Updating chart:', {
        chartType: currentChartType,
        symbol: currentSymbol,
        startTime: currentStartTime,
        endTime: currentEndTime,
      });

      await chartState.chart.update_chart(
        currentChartType,
        currentSymbol,
        BigInt(currentStartTime),
        BigInt(currentEndTime)
      );

      console.log('[useWasmChart] Chart updated successfully');
      return true;
    } catch (error) {
      console.error('[useWasmChart] Chart update failed:', error);
      
      if (mountedRef.current) {
        setChartState(prev => ({
          ...prev,
          error: `Update failed: ${error}`,
        }));
      }
      
      return false;
    }
  }, [chartState.chart, chartState.isInitialized, storeChartType, storeSymbol, storeStartTime, storeEndTime]);

  /**
   * Render a frame
   */
  const render = useCallback(async (): Promise<boolean> => {
    if (!chartState.chart || !chartState.isInitialized) return false;

    try {
      // Note: Direct rendering is handled in the render loop
      // This method is kept for manual render calls only
      return true;
    } catch (error) {
      console.error('[useWasmChart] Render failed:', error);
      return false;
    }
  }, [chartState.chart, chartState.isInitialized]);

  /**
   * Resize the chart
   */
  const resize = useCallback((width: number, height: number) => {
    if (!chartState.chart || !chartState.isInitialized) return;

    try {
      chartState.chart.resize(width, height);
    } catch (error) {
      console.error('[useWasmChart] Resize failed:', error);
    }
  }, [chartState.chart, chartState.isInitialized]);

  /**
   * Update configuration
   */
  const updateConfig = useCallback(async (configJson: string): Promise<boolean> => {
    if (!chartState.chart || !chartState.isInitialized) return false;

    try {
      chartState.chart.update_config(configJson);
      return true;
    } catch (error) {
      console.error('[useWasmChart] Config update failed:', error);
      return false;
    }
  }, [chartState.chart, chartState.isInitialized]);

  /**
   * Get current configuration
   */
  const getConfig = useCallback((): string => {
    if (!chartState.chart || !chartState.isInitialized) return '{}';

    try {
      return chartState.chart.get_config();
    } catch (error) {
      console.error('[useWasmChart] Get config failed:', error);
      return '{}';
    }
  }, [chartState.chart, chartState.isInitialized]);

  /**
   * Get performance stats
   */
  const getStats = useCallback((): string => {
    if (!chartState.chart || !chartState.isInitialized) return '{}';

    try {
      return chartState.chart.get_stats();
    } catch (error) {
      console.error('[useWasmChart] Get stats failed:', error);
      return '{}';
    }
  }, [chartState.chart, chartState.isInitialized]);

  /**
   * Destroy the chart
   */
  const destroy = useCallback(() => {
    if (!chartState.chart) return;

    try {
      chartState.chart.destroy();
    } catch (error) {
      console.error('[useWasmChart] Destroy failed:', error);
    }

    setChartState({
      chart: null,
      isInitialized: false,
      isLoading: false,
      error: null,
      fps: 0,
      frameTime: 0,
    });
  }, [chartState.chart]);

  // Automatic store state subscription with debouncing
  useEffect(() => {
    if (!enableAutoSync || !chartState.isInitialized) return;

    // Clear existing debounce timer
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    // Debounced update
    debounceRef.current = setTimeout(() => {
      if (mountedRef.current) {
        updateChart();
      }
    }, debounceMs);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [storeSymbol, storeChartType, storeStartTime, storeEndTime, enableAutoSync, chartState.isInitialized, debounceMs, updateChart]);

  // Render loop for continuous updates
  useEffect(() => {
    if (!chartState.isInitialized || !chartState.chart) return;

    let frameCount = 0;
    
    const renderLoop = async () => {
      if (!mountedRef.current || !chartState.chart || !chartState.isInitialized) return;
      
      try {
        // Call render directly on the chart object to avoid borrowing issues
        if (frameCount % 60 === 0) {
          console.log('[useWasmChart] Calling chart.render()');
        }
        chartState.chart.render();
        
        // Only get stats every 60 frames (roughly once per second)
        frameCount++;
        if (frameCount % 60 === 0) {
          console.log('[useWasmChart] Frame 60 reached, getting stats');
          try {
            const stats = chartState.chart.get_stats();
            const parsedStats = JSON.parse(stats);
            if (parsedStats.performance) {
              setChartState(prev => ({
                ...prev,
                fps: Math.round(1000 / parsedStats.performance.frame_time_ms),
                frameTime: parsedStats.performance.frame_time_ms,
              }));
            }
          } catch (e) {
            console.error('[useWasmChart] Error getting stats:', e);
          }
        }
      } catch (error) {
        // Only log render errors occasionally to avoid spam
        if (frameCount % 60 === 0) {
          console.error('[useWasmChart] Render error:', error);
        }
      }

      if (mountedRef.current) {
        animationFrameRef.current = requestAnimationFrame(renderLoop);
      }
    };

    animationFrameRef.current = requestAnimationFrame(renderLoop);

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [chartState.isInitialized, chartState.chart]);

  // Cleanup on unmount
  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
      if (debounceRef.current) clearTimeout(debounceRef.current);
      if (animationFrameRef.current) cancelAnimationFrame(animationFrameRef.current);
      if (chartState.chart) {
        destroy();
      }
    };
  }, []); // Only run on mount/unmount

  // API object
  const api: WasmChartAPI = {
    initialize,
    updateChart,
    render,
    resize,
    updateConfig,
    getConfig,
    getStats,
    destroy,
  };

  return [chartState, api];
}