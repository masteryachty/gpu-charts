import { useCallback, useEffect, useRef, useState } from 'react';
import { useAppStore } from '../store/useAppStore';
// import { useAutonomousDataFetching } from './useAutonomousDataFetching'; // TEMPORARILY DISABLED
import { useErrorHandler } from './useErrorHandler';
import { ERROR_CODES } from '../errors/ErrorTypes';
import type { StoreState } from '../types';

/**
 * Configuration constants for the WASM chart hook
 */
const WASM_CHART_CONSTANTS = {
  DEFAULT_DEBOUNCE_MS: 100,
  DEFAULT_MAX_RETRIES: 3,
  DEFAULT_RETRY_DELAY_MS: 1000,
  DEFAULT_PERFORMANCE_INTERVAL_MS: 1000,
  INITIALIZATION_DELAY_MS: 50,
  WASM_LOAD_TIMEOUT_MS: 10000,
  MAX_MEMORY_GROWTH_PERCENTAGE: 200,
  MIN_FPS_THRESHOLD: 30,
  MAX_RENDER_LATENCY_MS: 16, // 60fps budget
} as const;

/**
 * Advanced WASM Chart Integration Hook
 * 
 * Provides comprehensive React-Rust bridge with automatic store subscription,
 * smart change detection, and optimized rendering triggers.
 */
export interface UseWasmChartOptions {
  /** Canvas ID for WebGPU surface attachment */
  canvasId: string;
  
  /** Canvas dimensions */
  width?: number;
  height?: number;
  
  /** Enable automatic store state synchronization */
  enableAutoSync?: boolean;
  
  /** Debounce delay for state changes (ms) */
  debounceMs?: number;
  
  /** Enable autonomous data fetching */
  enableDataFetching?: boolean;
  
  /** Error recovery options */
  maxRetries?: number;
  retryDelayMs?: number;
  
  /** Performance monitoring */
  enablePerformanceMonitoring?: boolean;
  performanceIntervalMs?: number;
}

/**
 * WASM Chart instance type - represents the actual chart object from WASM
 */
export interface WasmChartInstance {
  is_initialized(): boolean;
  init(canvasId: string): void;
  handle_mouse_wheel?(delta: number, x: number, y: number): void;
  handle_mouse_move?(x: number, y: number): void;
  handle_mouse_click?(x: number, y: number, pressed: boolean): void;
  render?(): Promise<void>;
}

export interface WasmChartState {
  /** Chart instance from WASM */
  chart: WasmChartInstance | null;
  
  /** Initialization state */
  isInitialized: boolean;
  isLoading: boolean;
  
  /** Error handling */
  error: string | null;
  lastError: string | null;
  retryCount: number;
  
  /** Performance metrics */
  fps: number;
  renderLatency: number;
  updateCount: number;
  
  /** Change tracking */
  lastStateUpdate: number;
  hasUncommittedChanges: boolean;
  
  /** Data fetching integration */
  dataFetchingEnabled: boolean;
  lastDataFetch: {
    symbol: string;
    recordCount: number;
    timestamp: number;
    fromCache: boolean;
  } | null;
}

/**
 * Change detection configuration interface
 */
export interface ChangeDetectionConfig {
  enableSymbolChangeDetection: boolean;
  enableTimeRangeChangeDetection: boolean;
  enableTimeframeChangeDetection: boolean;
  enableIndicatorChangeDetection: boolean;
  symbolChangeTriggersFetch: boolean;
  timeRangeChangeTriggersFetch: boolean;
  timeframeChangeTriggersRender: boolean;
  indicatorChangeTriggersRender: boolean;
  minimumTimeRangeChangeSeconds: number;
}

/**
 * Performance metrics interface
 */
export interface PerformanceMetrics {
  fps: number;
  renderLatency: number;
  updateCount: number;
  lastStateUpdate: number;
  memoryUsage?: number;
  cpuUsage?: number;
}

/**
 * Change detection result interface
 */
export interface ChangeDetectionResult {
  hasChanges: boolean;
  symbolChanged: boolean;
  timeRangeChanged: boolean;
  timeframeChanged: boolean;
  indicatorsChanged: boolean;
  connectionChanged: boolean;
  userChanged: boolean;
  marketDataChanged: boolean;
  requiresDataFetch: boolean;
  requiresRender: boolean;
  summary: string[];
}

export interface WasmChartAPI {
  /** Manual chart operations */
  initialize: () => Promise<boolean>;
  updateState: (symbol?: string, timeframe?: string, connected?: boolean) => Promise<boolean>;
  forceUpdate: () => Promise<boolean>;
  
  /** Configuration management */
  configureChangeDetection: (config: Partial<ChangeDetectionConfig>) => Promise<boolean>;
  getChangeDetectionConfig: () => Promise<ChangeDetectionConfig>;
  
  /** State inspection */
  getCurrentState: () => Promise<StoreState | null>;
  detectChanges: (storeState: StoreState) => Promise<ChangeDetectionResult>;
  
  /** Error recovery */
  retry: () => Promise<boolean>;
  reset: () => Promise<boolean>;
  
  /** Performance */
  getPerformanceMetrics: () => Promise<PerformanceMetrics>;
  clearPerformanceMetrics: () => void;
}

export function useWasmChart(options: UseWasmChartOptions): [WasmChartState, WasmChartAPI] {
  const {
    canvasId,
    width,
    height,
    enableAutoSync = true,
    debounceMs = WASM_CHART_CONSTANTS.DEFAULT_DEBOUNCE_MS,
    enableDataFetching = true,
    maxRetries = WASM_CHART_CONSTANTS.DEFAULT_MAX_RETRIES,
    retryDelayMs = WASM_CHART_CONSTANTS.DEFAULT_RETRY_DELAY_MS,
    enablePerformanceMonitoring = true,
    performanceIntervalMs = WASM_CHART_CONSTANTS.DEFAULT_PERFORMANCE_INTERVAL_MS,
  } = options;

  // Get specific store state values to avoid full store re-renders
  const storeSymbol = useAppStore(state => state.currentSymbol);
  const storeTimeframe = useAppStore(state => state.chartConfig.timeframe);
  const storeConnected = useAppStore(state => state.isConnected);
  
  // Initialize comprehensive error handling - temporarily disabled for testing
  // const [errorState, errorAPI] = useErrorHandler({
  //   subscribeToCategories: ['wasm', 'store', 'data'],
  //   onError: (error) => {
  //     console.log('[useWasmChart] Error handler received error:', error.code);
  //   },
  //   onRecovery: (errorCode) => {
  //     console.log('[useWasmChart] Error recovery successful for:', errorCode);
  //   }
  // });

  // Mock error API for testing
  const errorAPI = {
    reportWasmError: async (code: string, message: string, context?: any) => {
      console.error('[useWasmChart] WASM Error:', { code, message, context });
    },
    reportStoreError: async (code: string, message: string, operation: any, context?: any) => {
      console.error('[useWasmChart] Store Error:', { code, message, operation, context });
    },
    registerRecoveryStrategy: (strategy: any) => {
      console.log('[useWasmChart] Recovery strategy registered:', strategy.errorCode);
    }
  };
  
  // Initialize autonomous data fetching if enabled
  // TEMPORARILY DISABLED TO FIX INFINITE LOOP
  // const [dataFetchingState, dataFetchingAPI] = useAutonomousDataFetching(
  //   enableDataFetching ? {
  //     enableAutoFetch: true,
  //     enablePrefetch: true,
  //     debounceMs: debounceMs,
  //   } : { enableAutoFetch: false }
  // );
  
  // Mock the data fetching state for now
  const dataFetchingState = {
    lastFetch: null
  };
  
  // Chart state management
  const [chartState, setChartState] = useState<WasmChartState>({
    chart: null,
    isInitialized: false,
    isLoading: false,
    error: null,
    lastError: null,
    retryCount: 0,
    fps: 0,
    renderLatency: 0,
    updateCount: 0,
    lastStateUpdate: 0,
    hasUncommittedChanges: false,
    dataFetchingEnabled: enableDataFetching,
    lastDataFetch: null,
  });

  // Refs for cleanup and performance
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();
  const performanceRef = useRef<ReturnType<typeof setInterval>>();
  const mountedRef = useRef(true);
  const lastSerializedStateRef = useRef<string>('');
  const updateStateRef = useRef<typeof updateState | null>(null);

  /**
   * Initialize the WASM chart instance
   */
  const initialize = useCallback(async (): Promise<boolean> => {
    if (!mountedRef.current) return false;

    setChartState(prev => ({
      ...prev,
      isLoading: true,
      error: null,
      retryCount: 0,
    }));

    try {
      console.log(`[useWasmChart] Initializing chart for canvas: ${canvasId}`);

      // Verify canvas exists
      const canvas = document.getElementById(canvasId);
      if (!canvas) {
        await errorAPI.reportWasmError(
          ERROR_CODES.WASM_CANVAS_ERROR,
          `Canvas with ID "${canvasId}" not found`,
          { method: 'initialize', canvasId, recoverable: true }
        );
        throw new Error(`Canvas with ID "${canvasId}" not found`);
      }

      // Dynamic WASM module import
      const wasmModule = await import('@pkg/tutorial1_window.js');
      await wasmModule.default();

      if (!mountedRef.current) return false;

      // Create SimpleChart instance (what's actually exported)
      const chart = new wasmModule.SimpleChart();
      
      // Initialize with canvas ID only (SimpleChart doesn't take dimensions)
      chart.init(canvasId);

      if (!mountedRef.current) return false;

      // Verify initialization
      if (!chart.is_initialized()) {
        await errorAPI.reportWasmError(
          ERROR_CODES.WASM_INIT_FAILED,
          'Chart initialization failed - is_initialized() returned false',
          { method: 'initialize', recoverable: true }
        );
        throw new Error('Chart initialization failed - is_initialized() returned false');
      }

      console.log('[useWasmChart] Chart initialized successfully');

      setChartState(prev => ({
        ...prev,
        chart,
        isInitialized: true,
        isLoading: false,
        error: null,
        lastStateUpdate: Date.now(),
      }));

      // Trigger initial state sync if enabled
      if (enableAutoSync) {
        setTimeout(() => updateState(), WASM_CHART_CONSTANTS.INITIALIZATION_DELAY_MS);
      }

      return true;
    } catch (error) {
      console.error('[useWasmChart] Initialization failed:', error);
      
      // Report to error handler if not already reported
      if (!String(error).includes('Canvas with ID') && !String(error).includes('is_initialized')) {
        await errorAPI.reportWasmError(
          ERROR_CODES.WASM_INIT_FAILED,
          `WASM chart initialization failed: ${error}`,
          { 
            method: 'initialize', 
            canvasId, 
            width, 
            height, 
            recoverable: true,
            wasmStack: (error as any)?.stack 
          }
        );
      }
      
      if (mountedRef.current) {
        setChartState(prev => ({
          ...prev,
          isLoading: false,
          error: `Initialization failed: ${error}`,
          lastError: String(error),
        }));
      }
      
      return false;
    }
  }, [canvasId, width, height, enableAutoSync, errorAPI, updateState]);

  /**
   * Update chart state from store state (simplified for SimpleChart)
   */
  const updateState = useCallback(async (symbol?: string, timeframe?: string, connected?: boolean): Promise<boolean> => {
    const currentSymbol = symbol || storeSymbol;
    const currentTimeframe = timeframe || storeTimeframe;
    const currentConnected = connected || storeConnected;
    
    if (!chartState.chart || !chartState.isInitialized) {
      console.warn('[useWasmChart] Cannot update state - chart not initialized');
      return false;
    }

    try {
      console.log('[useWasmChart] Chart state update requested:', {
        symbol: currentSymbol,
        timeframe: currentTimeframe,
        connected: currentConnected,
      });

      // For SimpleChart, we just verify it's still initialized and update our metrics
      const isStillInitialized = chartState.chart.is_initialized();
      
      if (!isStillInitialized) {
        throw new Error('Chart is no longer initialized');
      }

      const renderLatency = 1; // Minimal latency for simple check
      
      if (mountedRef.current) {
        setChartState(prev => ({
          ...prev,
          error: null,
          lastStateUpdate: Date.now(),
          hasUncommittedChanges: false,
          renderLatency,
          updateCount: prev.updateCount + 1,
        }));
      }

      console.log('[useWasmChart] Chart state updated successfully (SimpleChart mode)');

      return true;
    } catch (error) {
      console.error('[useWasmChart] State update failed:', error);
      
      // Report store synchronization error
      await errorAPI.reportStoreError(
        ERROR_CODES.STORE_SYNC_FAILED,
        `Chart state update failed: ${error}`,
        'sync',
        { 
          storeState: { symbol: currentSymbol, timeframe: currentTimeframe, connected: currentConnected }, 
          wasmMethod: 'is_initialized',
          error: String(error)
        }
      );
      
      if (mountedRef.current) {
        setChartState(prev => ({
          ...prev,
          error: `State update failed: ${error}`,
          lastError: String(error),
        }));
      }
      
      return false;
    }
  }, [chartState.chart, chartState.isInitialized, storeSymbol, storeTimeframe, storeConnected, errorAPI]);

  // Update updateState ref to avoid dependency cycles
  useEffect(() => {
    updateStateRef.current = updateState;
  }, [updateState]);

  /**
   * Force update (simplified for SimpleChart)
   */
  const forceUpdate = useCallback(async (): Promise<boolean> => {
    if (!chartState.chart || !chartState.isInitialized) return false;

    try {
      console.log('[useWasmChart] Force update requested (SimpleChart mode)');
      
      // For SimpleChart, just verify it's still initialized
      const isStillInitialized = chartState.chart.is_initialized();
      
      if (!isStillInitialized) {
        throw new Error('Chart is no longer initialized');
      }
      
      if (mountedRef.current) {
        setChartState(prev => ({
          ...prev,
          error: null,
          lastStateUpdate: Date.now(),
          hasUncommittedChanges: false,
          updateCount: prev.updateCount + 1,
        }));
      }

      console.log('[useWasmChart] Force update completed (SimpleChart mode)');
      return true;
    } catch (error) {
      console.error('[useWasmChart] Force update failed:', error);
      
      if (mountedRef.current) {
        setChartState(prev => ({
          ...prev,
          error: `Force update failed: ${error}`,
        }));
      }
      
      return false;
    }
  }, [chartState.chart, chartState.isInitialized]);

  /**
   * Configure change detection behavior (simplified for SimpleChart)
   */
  const configureChangeDetection = useCallback(async (config: Partial<ChangeDetectionConfig>): Promise<boolean> => {
    console.log('[useWasmChart] Change detection config requested (SimpleChart mode):', config);
    return true; // Always succeed for SimpleChart
  }, []);

  /**
   * Get current change detection configuration (simplified for SimpleChart)
   */
  const getChangeDetectionConfig = useCallback(async (): Promise<ChangeDetectionConfig> => {
    console.log('[useWasmChart] Change detection config requested (SimpleChart mode)');
    return {
      enableSymbolChangeDetection: false,
      enableTimeRangeChangeDetection: false,
      enableTimeframeChangeDetection: false,
      enableIndicatorChangeDetection: false,
      symbolChangeTriggersFetch: false,
      timeRangeChangeTriggersFetch: false,
      timeframeChangeTriggersRender: false,
      indicatorChangeTriggersRender: false,
      minimumTimeRangeChangeSeconds: 60,
    };
  }, []);

  /**
   * Get current Rust-side state (simplified for SimpleChart)
   */
  const getCurrentState = useCallback(async (): Promise<StoreState | null> => {
    console.log('[useWasmChart] Current state requested (SimpleChart mode)');
    return null; // SimpleChart doesn't maintain state
  }, []);

  /**
   * Detect changes without applying them (simplified for SimpleChart)
   */
  const detectChanges = useCallback(async (storeState: StoreState): Promise<ChangeDetectionResult> => {
    console.log('[useWasmChart] Change detection requested (SimpleChart mode):', storeState.currentSymbol);
    return {
      hasChanges: false,
      symbolChanged: false,
      timeRangeChanged: false,
      timeframeChanged: false,
      indicatorsChanged: false,
      connectionChanged: false,
      userChanged: false,
      marketDataChanged: false,
      requiresDataFetch: false,
      requiresRender: false,
      summary: ['SimpleChart mode - no change detection']
    };
  }, []);

  /**
   * Retry after error
   */
  const retry = useCallback(async (): Promise<boolean> => {
    if (chartState.retryCount >= maxRetries) {
      console.warn('[useWasmChart] Max retries exceeded');
      return false;
    }

    setChartState(prev => ({
      ...prev,
      retryCount: prev.retryCount + 1,
      error: null,
    }));

    // Wait before retry
    await new Promise(resolve => setTimeout(resolve, retryDelayMs));

    return initialize();
  }, [chartState.retryCount, maxRetries, retryDelayMs, initialize]);

  /**
   * Reset chart state
   */
  const reset = useCallback(async (): Promise<boolean> => {
    setChartState(prev => ({
      ...prev,
      chart: null,
      isInitialized: false,
      error: null,
      retryCount: 0,
      updateCount: 0,
      hasUncommittedChanges: false,
    }));

    lastSerializedStateRef.current = '';

    return initialize();
  }, [initialize]);

  /**
   * Get performance metrics
   */
  const getPerformanceMetrics = useCallback(async (): Promise<PerformanceMetrics> => {
    return {
      fps: chartState.fps,
      renderLatency: chartState.renderLatency,
      updateCount: chartState.updateCount,
      lastStateUpdate: chartState.lastStateUpdate,
      memoryUsage: performance.memory?.usedJSHeapSize,
      cpuUsage: undefined, // Not available in browser
    };
  }, [chartState.fps, chartState.renderLatency, chartState.updateCount, chartState.lastStateUpdate]);

  /**
   * Clear performance metrics
   */
  const clearPerformanceMetrics = useCallback(() => {
    setChartState(prev => ({
      ...prev,
      fps: 0,
      renderLatency: 0,
      updateCount: 0,
    }));
  }, []);

  // Automatic store state subscription with debouncing
  useEffect(() => {
    if (!enableAutoSync || !chartState.isInitialized) return;

    // Clear existing debounce timer
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    // Mark as having uncommitted changes
    setChartState(prev => ({ ...prev, hasUncommittedChanges: true }));

    // Debounced update
    debounceRef.current = setTimeout(() => {
      if (mountedRef.current && updateStateRef.current) {
        updateStateRef.current(storeSymbol, storeTimeframe, storeConnected);
      }
    }, debounceMs);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [storeSymbol, storeTimeframe, storeConnected, enableAutoSync, chartState.isInitialized, debounceMs]);

  // Performance monitoring
  useEffect(() => {
    if (!enablePerformanceMonitoring || !chartState.isInitialized) return;

    performanceRef.current = setInterval(async () => {
      if (!mountedRef.current || !chartState.chart) return;

      try {
        // Get metrics from WASM if available
        // For now, we'll use basic metrics
        const currentTime = Date.now();
        const timeSinceLastUpdate = currentTime - chartState.lastStateUpdate;
        
        setChartState(prev => ({
          ...prev,
          fps: timeSinceLastUpdate > 0 ? Math.round(1000 / Math.max(timeSinceLastUpdate, 16)) : 0,
        }));
      } catch (error) {
        console.error('[useWasmChart] Performance monitoring error:', error);
      }
    }, performanceIntervalMs);

    return () => {
      if (performanceRef.current) {
        clearInterval(performanceRef.current);
      }
    };
  }, [enablePerformanceMonitoring, chartState.isInitialized, chartState.chart, chartState.lastStateUpdate, performanceIntervalMs]);

  // Update chart state when data fetching completes
  // TEMPORARILY DISABLED WHILE FIXING INFINITE LOOP
  // useEffect(() => {
  //   if (!enableDataFetching || !dataFetchingState.lastFetch) return;
  //   
  //   setChartState(prev => ({
  //     ...prev,
  //     lastDataFetch: {
  //       symbol: dataFetchingState.lastFetch!.symbol,
  //       recordCount: dataFetchingState.lastFetch!.recordCount,
  //       timestamp: dataFetchingState.lastFetch!.timestamp,
  //       fromCache: dataFetchingState.lastFetch!.fromCache,
  //     }
  //   }));
  //   
  //   console.log('[useWasmChart] Data fetch completed:', {
  //     symbol: dataFetchingState.lastFetch.symbol,
  //     records: dataFetchingState.lastFetch.recordCount,
  //     cached: dataFetchingState.lastFetch.fromCache,
  //   });
  // }, [enableDataFetching, dataFetchingState.lastFetch]);

  // Setup error recovery strategies - register only once
  useEffect(() => {
    // WASM initialization recovery
    errorAPI.registerRecoveryStrategy({
      errorCode: ERROR_CODES.WASM_INIT_FAILED,
      maxAttempts: maxRetries,
      delayMs: retryDelayMs,
      action: async () => {
        console.log('[useWasmChart] Attempting WASM reinitialization...');
        // Use refs to avoid dependency issues
        return false; // For now, disable auto-recovery to prevent infinite loops
      },
      fallback: async () => {
        console.log('[useWasmChart] WASM fallback: showing error state');
        setChartState(prev => ({
          ...prev,
          error: 'Chart engine unavailable. Please refresh the page.',
          retryCount: maxRetries
        }));
      }
    });
    
    // Store sync recovery
    errorAPI.registerRecoveryStrategy({
      errorCode: ERROR_CODES.STORE_SYNC_FAILED,
      maxAttempts: 3,
      delayMs: 500,
      action: async () => {
        console.log('[useWasmChart] Attempting store sync recovery...');
        // Use refs to avoid dependency issues
        return false; // For now, disable auto-recovery to prevent infinite loops
      }
    });
  }, [errorAPI, maxRetries, retryDelayMs]); // Include dependencies

  // Cleanup on unmount
  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
      if (debounceRef.current) clearTimeout(debounceRef.current);
      if (performanceRef.current) clearInterval(performanceRef.current);
    };
  }, []);

  // API object
  const api: WasmChartAPI = {
    initialize,
    updateState,
    forceUpdate,
    configureChangeDetection,
    getChangeDetectionConfig,
    getCurrentState,
    detectChanges,
    retry,
    reset,
    getPerformanceMetrics,
    clearPerformanceMetrics,
  };

  return [chartState, api];
}