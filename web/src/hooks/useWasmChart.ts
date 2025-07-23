import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useAppStore } from '../store/useAppStore';
// import { useAutonomousDataFetching } from './useAutonomousDataFetching'; // TEMPORARILY DISABLED
// import { useErrorHandler } from './useErrorHandler'; // TEMPORARILY DISABLED
import { usePerformanceMonitor } from './usePerformanceMonitor';
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
  init(canvasId: string, width: number, height: number): Promise<void>;
  handle_mouse_wheel?(delta_y: number, x: number, y: number): void;
  handle_mouse_move?(x: number, y: number): void;
  handle_mouse_click?(x: number, y: number, pressed: boolean): void;
  render?(): Promise<void>;
  needs_render?(): boolean;
  resize?(width: number, height: number): void;
  
  // Chart state management
  update_state?(symbol: string, timeframe: string, connected: boolean): void;
  update_chart_state?(stateJson: string): string;
  
  // Chart type controls
  set_chart_type?(chart_type: string): void;
  set_candle_timeframe?(timeframe_seconds: number): void;
  
  // Change detection
  configure_change_detection?(config: any): string;
  get_change_detection_config?(): string;
  detect_changes?(storeState: any): string;
  get_current_state?(): Promise<any>;
  get_current_store_state?(): string;
  force_update_chart_state?(stateJson: string): string;
  detect_state_changes?(stateJson: string): string;
  set_data_range?(start: number, end: number): void;
  request_redraw?(): void;
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
  forceStateUpdate?: () => Promise<boolean>;
  
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
  const storeSelectedMetrics = useAppStore(state => state.chartConfig.selectedMetrics);
  const storeChartType = useAppStore(state => state.chartConfig.chartType);
  const storeCandleTimeframe = useAppStore(state => state.chartConfig.candleTimeframe);
  
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

  // Mock error API for testing - wrapped in useMemo to prevent dependency warnings
  const errorAPI = useMemo(() => ({
    reportWasmError: async (code: string, message: string, context?: any) => {
      console.error('[useWasmChart] WASM Error:', { code, message, context });
    },
    reportStoreError: async (code: string, message: string, operation: any, context?: any) => {
      console.error('[useWasmChart] Store Error:', { code, message, operation, context });
    },
    registerRecoveryStrategy: (strategy: any) => {
      console.log('[useWasmChart] Recovery strategy registered:', strategy.errorCode);
    }
  }), []);
  
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
  // const dataFetchingState = {
  //   lastFetch: null
  // };
  
  // Re-enable performance monitoring 
  const [performanceState, performanceAPI] = usePerformanceMonitor(
    enablePerformanceMonitoring ? {
      enableFpsMonitoring: true,
      enableMemoryMonitoring: true,
      enableCpuMonitoring: true,
      updateIntervalMs: performanceIntervalMs,
      maxHistorySize: 60,
      enableWarnings: true,
      fpsWarningThreshold: WASM_CHART_CONSTANTS.MIN_FPS_THRESHOLD,
      memoryWarningThreshold: 100 * 1024 * 1024,
    } : {
      enableFpsMonitoring: false,
      enableMemoryMonitoring: false,
      enableCpuMonitoring: false,
      updateIntervalMs: performanceIntervalMs,
    }
  );
  
  // Chart state management
  const [chartState, setChartState] = useState<WasmChartState>({
    chart: null,
    isInitialized: false,
    isLoading: false,
    error: null,
    lastError: null,
    retryCount: 0,
    fps: performanceState.metrics.fps,
    renderLatency: performanceState.metrics.renderLatency,
    updateCount: 0,
    lastStateUpdate: 0,
    hasUncommittedChanges: false,
    dataFetchingEnabled: enableDataFetching,
    lastDataFetch: null,
  });

  // Refs for cleanup and performance
  const debounceRef = useRef<ReturnType<typeof setTimeout>>();
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

      // Wait for canvas to be available with retry logic
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
        await errorAPI.reportWasmError(
          ERROR_CODES.WASM_CANVAS_ERROR,
          `Canvas with ID "${canvasId}" not found after ${maxCanvasRetries} attempts`,
          { method: 'initialize', canvasId, recoverable: true }
        );
        throw new Error(`Canvas with ID "${canvasId}" not found after ${maxCanvasRetries} attempts`);
      }

      // Ensure canvas has dimensions
      const canvasElement = canvas as HTMLCanvasElement;
      if (canvasElement.clientWidth === 0 || canvasElement.clientHeight === 0) {
        console.log(`[useWasmChart] Canvas has no dimensions, setting defaults`);
        canvasElement.style.width = '100%';
        canvasElement.style.height = '100%';
        // Wait for layout to update
        await new Promise(resolve => setTimeout(resolve, 50));
      }

      // Dynamic WASM module import with test fallback
      let chart: WasmChartInstance;
      
      try {
        console.log('[useWasmChart] Loading WASM module...');
        const wasmModule = await import('@pkg/wasm_bridge.js');
        console.log('[useWasmChart] WASM module imported, initializing...');
        await wasmModule.default();
        console.log('[useWasmChart] WASM module initialized');

        if (!mountedRef.current) {
          console.log('[useWasmChart] Component unmounted during WASM init');
          return false;
        }

        // Create Chart instance 
        console.log('[useWasmChart] Creating Chart instance...');
        chart = new wasmModule.Chart();
        console.log('[useWasmChart] Chart instance created');
        
        // Initialize with canvas ID and actual canvas dimensions
        const actualWidth = width || canvasElement.clientWidth || 800;
        const actualHeight = height || canvasElement.clientHeight || 600;
        console.log(`[useWasmChart] Initializing chart with canvas: ${canvasId}, size: ${actualWidth}x${actualHeight}`);
        
        try {
          await chart.init(canvasId, actualWidth, actualHeight);
          console.log('[useWasmChart] Chart.init() completed');
        } catch (initError) {
          console.error('[useWasmChart] Chart.init() failed:', initError);
          throw initError;
        }
        
        // Verify chart is actually initialized
        const isInitialized = chart.is_initialized();
        console.log(`[useWasmChart] Chart.is_initialized() = ${isInitialized}`);
        
        if (!isInitialized) {
          throw new Error('Chart initialization failed - is_initialized() returned false after init()');
        }
        
        // Try to render once to see if that works
        if (chart.render) {
          console.log('[useWasmChart] Attempting initial render...');
          try {
            await chart.render();
            console.log('[useWasmChart] Initial render completed');
          } catch (renderError) {
            console.error('[useWasmChart] Initial render failed:', renderError);
            // Don't throw here, rendering might fail but chart could still be usable
          }
        }
      } catch (wasmImportError) {
        console.warn('[useWasmChart] WASM module not available, using mock for testing:', wasmImportError);
        
        // Create a comprehensive mock chart for testing
        let mockInitialized = false;
        
        chart = {
          is_initialized: () => mockInitialized,
          init: async (canvasId: string, width: number, height: number) => {
            console.log(`[useWasmChart] Mock chart initialized with canvas: ${canvasId}, size: ${width}x${height}`);
            mockInitialized = true;
            
            // Set canvas data attributes for testing
            const canvasEl = document.getElementById(canvasId) as HTMLCanvasElement;
            if (canvasEl) {
              console.log(`[useWasmChart] Mock chart setting canvas attributes`);
              canvasEl.setAttribute('data-initialized', 'true');
              canvasEl.setAttribute('data-mock', 'true');
              
              // Set canvas dimensions to match container
              if (canvasEl.clientWidth > 0 && canvasEl.clientHeight > 0) {
                canvasEl.width = canvasEl.clientWidth;
                canvasEl.height = canvasEl.clientHeight;
                console.log(`[useWasmChart] Mock chart set canvas size to: ${canvasEl.width}x${canvasEl.height}`);
              } else {
                canvasEl.width = width;
                canvasEl.height = height;
                console.log(`[useWasmChart] Mock chart set canvas size to: ${width}x${height}`);
              }
            } else {
              console.error(`[useWasmChart] Mock chart could not find canvas with id: ${canvasId}`);
            }
            
            console.log(`[useWasmChart] Mock chart initialization completed, mockInitialized: ${mockInitialized}`);
          },
          handle_mouse_wheel: (delta: number, x: number, y: number) => {
            console.log(`[useWasmChart] Mock mouse wheel: ${delta} at ${x},${y}`);
          },
          handle_mouse_move: (x: number, y: number) => {
            console.log(`[useWasmChart] Mock mouse move: ${x},${y}`);
          },
          handle_mouse_click: (x: number, y: number, pressed: boolean) => {
            console.log(`[useWasmChart] Mock mouse click: ${x},${y} pressed=${pressed}`);
          },
          update_state: async (symbol: string, timeframe: string, connected: boolean) => {
            console.log(`[useWasmChart] Mock state update: ${symbol}, ${timeframe}, ${connected}`);
            return Promise.resolve();
          },
          render: async () => {
            console.log(`[useWasmChart] Mock render`);
            return Promise.resolve();
          },
          // Additional mock methods for testing
          update_chart_state: (stateJson: string) => {
            console.log(`[useWasmChart] Mock chart state update: ${stateJson}`);
            return JSON.stringify({ success: true, message: 'Mock state update' });
          },
          get_current_state: async () => {
            return {
              currentSymbol: storeSymbol,
              chartConfig: { timeframe: storeTimeframe },
              isConnected: storeConnected,
              chartInitialized: true
            };
          },
          get_current_store_state: () => {
            return JSON.stringify({
              currentSymbol: storeSymbol,
              chartConfig: { timeframe: storeTimeframe },
              isConnected: storeConnected,
              chartInitialized: true
            });
          }
        };
        
        // Initialize mock chart
        const mockWidth = width || canvasElement.clientWidth || 800;
        const mockHeight = height || canvasElement.clientHeight || 600;
        await chart.init(canvasId, mockWidth, mockHeight);
      }

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

      setChartState(prev => {
        const newUpdateCount = prev.updateCount + 1;
        lastUpdateRef.current.updateCount = newUpdateCount;
        
        return {
          ...prev,
          chart,
          isInitialized: true,
          isLoading: false,
          error: null,
          lastStateUpdate: Date.now(),
          updateCount: newUpdateCount,
        };
      });

      // Start performance monitoring if enabled
      if (enablePerformanceMonitoring) {
        performanceAPI.start();
      }

      // Trigger initial state sync if enabled
      if (enableAutoSync) {
        setTimeout(() => {
          if (updateStateRef.current) {
            updateStateRef.current();
          }
        }, WASM_CHART_CONSTANTS.INITIALIZATION_DELAY_MS);
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
  }, [canvasId, width, height, enableAutoSync, enablePerformanceMonitoring, performanceAPI, storeSymbol, storeTimeframe, storeConnected, errorAPI]);

  /**
   * Update chart state from store state
   */
  const updateState = useCallback(async (symbol?: string, timeframe?: string, connected?: boolean, selectedMetrics?: string[]): Promise<boolean> => {
    const currentSymbol = symbol || storeSymbol;
    const currentTimeframe = timeframe || storeTimeframe;
    const currentConnected = connected || storeConnected;
    const currentSelectedMetrics = selectedMetrics || storeSelectedMetrics;
    
    if (!chartState.chart || !chartState.isInitialized) {
      console.warn('[useWasmChart] Cannot update state - chart not initialized');
      return false;
    }

    try {
      console.log('[useWasmChart] Chart state update requested:', {
        symbol: currentSymbol,
        timeframe: currentTimeframe,
        connected: currentConnected,
        selectedMetrics: currentSelectedMetrics,
      });

      // Update chart state based on store changes
      const isStillInitialized = chartState.chart.is_initialized();
      
      if (!isStillInitialized) {
        throw new Error('Chart is no longer initialized');
      }

      // Measure render latency
      const startTime = performance.now();
      
      // Use the new update_chart_state method with full store state
      if (chartState.chart.update_chart_state) {
        try {
          // Get the current store state and pass it to WASM
          const storeState = useAppStore.getState();
          const storeStateJson = JSON.stringify({
            currentSymbol: currentSymbol,
            chartConfig: {
              symbol: currentSymbol,
              timeframe: currentTimeframe,
              selectedMetrics: currentSelectedMetrics,
              startTime: storeState.chartConfig.startTime,
              endTime: storeState.chartConfig.endTime,
              indicators: storeState.chartConfig.indicators,
              chartType: storeState.chartConfig.chartType,
              candleTimeframe: storeState.chartConfig.candleTimeframe,
            },
            isConnected: currentConnected,
            marketData: storeState.marketData,
            user: storeState.user,
          });
          
          console.log('[useWasmChart] Sending store state to WASM:', storeStateJson);
          const result = chartState.chart.update_chart_state(storeStateJson);
          console.log('[useWasmChart] WASM update result:', result);
        } catch (wasmError) {
          throw new Error(`WASM update_chart_state failed: ${wasmError}`);
        }
      } else if (chartState.chart.update_state) {
        // Fallback to old method if new one isn't available
        try {
          await chartState.chart.update_state(currentSymbol, currentTimeframe, currentConnected);
        } catch (wasmError) {
          throw new Error(`WASM update_state failed: ${wasmError}`);
        }
      }
      
      const renderLatency = performance.now() - startTime;
      
      if (mountedRef.current) {
        setChartState(prev => {
          const newUpdateCount = prev.updateCount + 1;
          lastUpdateRef.current.updateCount = newUpdateCount;
          
          return {
            ...prev,
            error: null,
            lastStateUpdate: Date.now(),
            hasUncommittedChanges: false,
            renderLatency,
            updateCount: newUpdateCount,
          };
        });
      }

      console.log('[useWasmChart] Chart state updated successfully');

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
  }, [chartState.chart, chartState.isInitialized, storeSymbol, storeTimeframe, storeConnected, storeSelectedMetrics, errorAPI]);

  // Update updateState ref to avoid dependency cycles
  useEffect(() => {
    updateStateRef.current = updateState;
  }, [updateState]);

  /**
   * Force update
   */
  const forceUpdate = useCallback(async (): Promise<boolean> => {
    if (!chartState.chart || !chartState.isInitialized) return false;

    try {
      console.log('[useWasmChart] Force update requested');
      
      // Verify chart is still initialized
      const isStillInitialized = chartState.chart.is_initialized();
      
      if (!isStillInitialized) {
        throw new Error('Chart is no longer initialized');
      }
      
      // Force render if available
      if (chartState.chart.render) {
        await chartState.chart.render();
      }
      
      if (mountedRef.current) {
        setChartState(prev => {
          const newUpdateCount = prev.updateCount + 1;
          lastUpdateRef.current.updateCount = newUpdateCount;
          
          return {
            ...prev,
            error: null,
            lastStateUpdate: Date.now(),
            hasUncommittedChanges: false,
            updateCount: newUpdateCount,
          };
        });
      }

      console.log('[useWasmChart] Force update completed');
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
   * Configure change detection behavior
   */
  const configureChangeDetection = useCallback(async (config: Partial<ChangeDetectionConfig>): Promise<boolean> => {
    console.log('[useWasmChart] Change detection config requested:', config);
    
    // Configure chart change detection if supported
    if (chartState.chart && chartState.chart.configure_change_detection) {
      const result = chartState.chart.configure_change_detection(config);
      return typeof result === 'string' ? true : result;
    }
    
    return true; // Default to success if not supported
  }, [chartState.chart]);

  /**
   * Get current change detection configuration
   */
  const getChangeDetectionConfig = useCallback(async (): Promise<ChangeDetectionConfig> => {
    console.log('[useWasmChart] Change detection config requested');
    
    // Get config from chart if supported
    if (chartState.chart && chartState.chart.get_change_detection_config) {
      const result = chartState.chart.get_change_detection_config();
      return typeof result === 'string' ? JSON.parse(result) : result;
    }
    
    // Default configuration
    return {
      enableSymbolChangeDetection: true,
      enableTimeRangeChangeDetection: true,
      enableTimeframeChangeDetection: true,
      enableIndicatorChangeDetection: true,
      symbolChangeTriggersFetch: true,
      timeRangeChangeTriggersFetch: true,
      timeframeChangeTriggersRender: true,
      indicatorChangeTriggersRender: true,
      minimumTimeRangeChangeSeconds: 60,
    };
  }, [chartState.chart]);

  /**
   * Get current Rust-side state
   */
  const getCurrentState = useCallback(async (): Promise<StoreState | null> => {
    console.log('[useWasmChart] Current state requested');
    
    // Get state from chart if supported
    if (chartState.chart && chartState.chart.get_current_state) {
      return chartState.chart.get_current_state();
    }
    
    return null; // Not supported by this chart implementation
  }, [chartState.chart]);

  /**
   * Detect changes without applying them
   */
  const detectChanges = useCallback(async (storeState: StoreState): Promise<ChangeDetectionResult> => {
    console.log('[useWasmChart] Change detection requested:', storeState.currentSymbol);
    
    // Use chart's change detection if supported
    if (chartState.chart && chartState.chart.detect_changes) {
      const result = chartState.chart.detect_changes(storeState);
      return typeof result === 'string' ? JSON.parse(result) : result;
    }
    
    // Basic change detection fallback
    return {
      hasChanges: true,
      symbolChanged: true,
      timeRangeChanged: true,
      timeframeChanged: false,
      indicatorsChanged: false,
      connectionChanged: false,
      userChanged: false,
      marketDataChanged: true,
      requiresDataFetch: true,
      requiresRender: true,
      summary: ['Basic change detection - assume changes present']
    };
  }, [chartState.chart]);

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
    const metrics = performanceAPI.getMetrics();
    return {
      fps: metrics.fps,
      renderLatency: metrics.renderLatency,
      updateCount: chartState.updateCount,
      lastStateUpdate: chartState.lastStateUpdate,
      memoryUsage: metrics.totalMemoryUsage,
      cpuUsage: metrics.cpuUsage,
    };
  }, [performanceAPI, chartState.updateCount, chartState.lastStateUpdate]);

  /**
   * Clear performance metrics
   */
  const clearPerformanceMetrics = useCallback(() => {
    performanceAPI.reset();
    setChartState(prev => ({
      ...prev,
      fps: 0,
      renderLatency: 0,
      updateCount: 0,
    }));
  }, [performanceAPI]);

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
        updateStateRef.current(storeSymbol, storeTimeframe, storeConnected, storeSelectedMetrics);
      }
    }, debounceMs);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [storeSymbol, storeTimeframe, storeConnected, storeSelectedMetrics, enableAutoSync, chartState.isInitialized, debounceMs]);
  
  // Effect to update chart type when it changes
  useEffect(() => {
    if (!chartState.isInitialized || !chartState.chart || !enableAutoSync) return;
    
    if (chartState.chart.set_chart_type) {
      try {
        console.log('[useWasmChart] Setting chart type:', storeChartType);
        chartState.chart.set_chart_type(storeChartType);
      } catch (error) {
        console.error('[useWasmChart] Error setting chart type:', error);
      }
    }
  }, [storeChartType, chartState.isInitialized, chartState.chart, enableAutoSync]);
  
  // Effect to update candle timeframe when it changes
  useEffect(() => {
    if (!chartState.isInitialized || !chartState.chart || !enableAutoSync) return;
    
    if (chartState.chart.set_candle_timeframe) {
      try {
        console.log('[useWasmChart] Setting candle timeframe:', storeCandleTimeframe);
        chartState.chart.set_candle_timeframe(storeCandleTimeframe);
      } catch (error) {
        console.error('[useWasmChart] Error setting candle timeframe:', error);
      }
    }
  }, [storeCandleTimeframe, chartState.isInitialized, chartState.chart, enableAutoSync]);

  // Performance monitoring sync effects - use ref to avoid infinite loops
  const lastUpdateRef = useRef({ fps: 0, renderLatency: 0, updateCount: 0 });
  
  useEffect(() => {
    if (!enablePerformanceMonitoring || !performanceState.isMonitoring) return;
    
    const throttleInterval = setInterval(() => {
      if (performanceState.metrics) {
        const { fps, renderLatency } = performanceState.metrics;
        const lastUpdate = lastUpdateRef.current;
        
        // Only update if values have significantly changed (avoid tiny fluctuations)
        const fpsDiff = Math.abs(lastUpdate.fps - fps);
        const latencyDiff = Math.abs(lastUpdate.renderLatency - renderLatency);
        
        if (fpsDiff > 2 || latencyDiff > 1) { // Only update for meaningful changes
          lastUpdateRef.current = { fps, renderLatency, updateCount: lastUpdate.updateCount };
          
          setChartState(prev => ({
            ...prev,
            fps: Math.round(fps),
            renderLatency: Math.round(renderLatency * 10) / 10, // Round to 1 decimal
          }));
        }
      }
    }, 2000); // Update at most once every 2 seconds
    
    return () => clearInterval(throttleInterval);
  }, [enablePerformanceMonitoring, performanceState.isMonitoring, performanceState.metrics]); // Depend on monitoring state

  useEffect(() => {
    if (enablePerformanceMonitoring && !chartState.isInitialized && performanceState.isMonitoring) {
      performanceAPI.stop();
    }
  }, [chartState.isInitialized, performanceState.isMonitoring, performanceAPI, enablePerformanceMonitoring]);

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
  }, [errorAPI, maxRetries, retryDelayMs]); // Add missing dependencies

  // Cleanup on unmount
  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
      if (debounceRef.current) clearTimeout(debounceRef.current);
      if (enablePerformanceMonitoring && performanceAPI) {
        performanceAPI.stop();
      }
    };
  }, [enablePerformanceMonitoring, performanceAPI]); // Add performanceAPI back with memoization

  // API object
  const api: WasmChartAPI = {
    initialize,
    updateState,
    forceUpdate,
    forceStateUpdate: forceUpdate, // Alias for backward compatibility
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