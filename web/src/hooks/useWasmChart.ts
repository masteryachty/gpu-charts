import { useCallback, useRef, useState, useEffect } from 'react';
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
  const chartRef = useRef<Chart | null>(null);
  const animationFrameRef = useRef<number | null>(null);

  /**
   * Initialize the WASM chart instance
   */
  const initialize = useCallback(async (startTime: number, endTime: number): Promise<boolean> => {
    if (!mountedRef.current) return false;

    try {

      // Wait for canvas to be available with retry logic
      let canvas: HTMLElement | null = document.getElementById(canvasId);
      if (!canvas) {
        throw new Error(`Canvas with ID "${canvasId}" not found `);
      }

      const canvasElement = canvas as HTMLCanvasElement;
      if (canvasElement.clientWidth === 0 || canvasElement.clientHeight === 0) {
        canvasElement.style.width = '100%';
        canvasElement.style.height = '100%';
        // Wait for layout to update
        await new Promise(resolve => setTimeout(resolve, 50));
      }

      // Dynamic WASM module import with test fallback
      let chart: Chart;

      console.log("123");

      try {
        // Use preloaded WASM module if available, otherwise fall back to dynamic import
        let wasmModule;
        if (window.wasmPromise) {
          console.log('[useWasmChart] Using preloaded WASM module');
          wasmModule = await window.wasmPromise;
        } else {
          console.log('[useWasmChart] Falling back to dynamic WASM import');
          wasmModule = await import('@pkg/wasm_bridge.js');
          await wasmModule.default();
        }

        if (!mountedRef.current) {
          return false;
        }

        console.log("234");

        // Create Chart instance 
        chart = new wasmModule.Chart();
        console.log("345");

        // Initialize with canvas ID and actual canvas dimensions
        const actualWidth = width || canvasElement.clientWidth || 800;
        const actualHeight = height || canvasElement.clientHeight || 600;

        try {
          await chart.init(canvasId, actualWidth, actualHeight, startTime, endTime);
          console.log("456");

        } catch (initError) {
          throw initError;
        }

        try {
          chartRef.current = chart;
        } catch (initError) {
          throw initError;
        }

      } catch (wasmImportError) {
      }

      if (!mountedRef.current) return false;


      setChartState(prev => {
        return {
          ...prev,
          chart,
          isInitialized: true
        };
      });

      return true;
    } catch (error) {
      return false;
    }
  }, [canvasId, width, height]);


  // Cleanup on unmount
  useEffect(() => {
    return () => {
      mountedRef.current = false;
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, []);

  // API object
  const api: WasmChartAPI = {
    initialize,
  };

  return [chartState, api];
}