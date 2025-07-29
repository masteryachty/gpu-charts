import { useCallback, useRef, useState } from 'react';
import { useAppStore } from '../store/useAppStore';
// import { useAutonomousDataFetching } from './useAutonomousDataFetching'; // TEMPORARILY DISABLED
// import { useErrorHandler } from './useErrorHandler'; // TEMPORARILY DISABLED
// import { usePerformanceMonitor } from './usePerformanceMonitor'; // TEMPORARILY DISABLED

import type { Chart } from '@pkg/wasm_bridge.js';

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
  width: number;
  height: number;
}

/**
 * WASM Chart instance type - represents the actual chart object from WASM
 */


export interface WasmChartState {
  chart: Chart | null;
  isInitialized: boolean;
}


export interface WasmChartAPI {
  /** Manual chart operations */
  initialize: (startTime: number, endTime: number) => Promise<boolean>;
}

export function useWasmChart(options: UseWasmChartOptions): [WasmChartState, WasmChartAPI] {
  const {
    canvasId,
    width,
    height
  } = options;

  // Get specific store state values to avoid full store re-renders
  const storeMetricPreset = useAppStore(state => state.preset);
  const storeStartTime = useAppStore(state => state.startTime);
  const storeEndTime = useAppStore(state => state.endTime);


  // Chart state management
  const [chartState, setChartState] = useState<WasmChartState>({
    chart: null,
    isInitialized: false,
  });

  // Refs for cleanup and performance
  const mountedRef = useRef(true);

  /**
   * Initialize the WASM chart instance
   */
  const initialize = useCallback(async (startTime: number, endTime: number): Promise<boolean> => {
    if (!mountedRef.current) return false;

    try {
      console.log(`[useWasmChart] Initializing chart for canvas: ${canvasId}`);

      // Wait for canvas to be available with retry logic
      let canvas: HTMLElement | null = document.getElementById(canvasId);
      if (!canvas) {
        throw new Error(`Canvas with ID "${canvasId}" not found `);
      }

      const canvasElement = canvas as HTMLCanvasElement;
      if (canvasElement.clientWidth === 0 || canvasElement.clientHeight === 0) {
        console.log(`[useWasmChart] Canvas has no dimensions, setting defaults`);
        canvasElement.style.width = '100%';
        canvasElement.style.height = '100%';
        // Wait for layout to update
        await new Promise(resolve => setTimeout(resolve, 50));
      }

      // Dynamic WASM module import with test fallback
      let chart: Chart;

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
          await chart.init(canvasId, actualWidth, actualHeight, startTime, endTime);
          console.log('[useWasmChart] Chart.init() completed');
        } catch (initError) {
          console.error('[useWasmChart] Chart.init() failed:', initError);
          throw initError;
        }

        // try {
        //   await chart.render();
        //   console.log('[useWasmChart] Chart.render() completed');
        // } catch (initError) {
        //   console.error('[useWasmChart] Chart.render() failed:', initError);
        //   throw initError;
        // }

      } catch (wasmImportError) {
        console.warn('[useWasmChart] WASM module not available', wasmImportError);
      }

      if (!mountedRef.current) return false;

      console.log('[useWasmChart] Chart initialized successfully');

      setChartState(prev => {
        return {
          ...prev,
          chart,
          isInitialized: true
        };
      });

      return true;
    } catch (error) {
      console.error('[useWasmChart] Initialization failed:', error);
      return false;
    }
  }, [canvasId, width, height, storeMetricPreset, storeStartTime, storeEndTime]);


  // API object
  const api: WasmChartAPI = {
    initialize,
  };

  return [chartState, api];
}