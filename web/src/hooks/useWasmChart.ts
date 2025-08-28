import { useCallback, useRef, useState, useEffect } from 'react';
import { useAppStore } from '../store/useAppStore';
import { useLoading } from '../contexts/LoadingContext';
import { usePerformanceMonitoring } from '../utils/performanceMonitor';
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

  const { setLoading } = useLoading();
  const { trackWasmLoad, trackWebGPUInit, trackChartPerformance } = usePerformanceMonitoring();

  // Get specific store state values to avoid full store re-renders
  const storeMetricPreset = useAppStore(state => state.preset);
  const _storeStartTime = useAppStore(state => state.startTime);
  const _storeEndTime = useAppStore(state => state.endTime);


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

    // Set loading states
    setLoading('wasm', true);
    setLoading('data', true);

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
      const wasmLoadStart = performance.now();

      try {
        // Use preloaded WASM module if available, otherwise fall back to dynamic import
        let wasmModule;
        if (window.wasmPromise) {
              wasmModule = await window.wasmPromise;
        } else {
          console.log('[useWasmChart] Falling back to dynamic WASM import');
          wasmModule = await import('@pkg/wasm_bridge.js');
          await wasmModule.default();
        }

        const wasmLoadTime = performance.now() - wasmLoadStart;
        trackWasmLoad(wasmLoadTime);

        if (!mountedRef.current) {
          return false;
        }

  
        // Create Chart instance 
        chart = new wasmModule.Chart();
  
        // Initialize with canvas ID and actual canvas dimensions
        const actualWidth = width || canvasElement.clientWidth || 800;
        const actualHeight = height || canvasElement.clientHeight || 600;
        const webgpuInitStart = performance.now();

        try {
          await chart.init(canvasId, actualWidth, actualHeight, startTime, endTime);
          const webgpuInitTime = performance.now() - webgpuInitStart;
          trackWebGPUInit(webgpuInitTime, true);
    
        } catch (initError) {
          const webgpuInitTime = performance.now() - webgpuInitStart;
          const errorMessage = initError instanceof Error ? initError.message : String(initError);
          trackWebGPUInit(webgpuInitTime, false, errorMessage);
          throw initError;
        }

        try {
          // await chart.render();

          // Store chart ref
          chartRef.current = chart;

          // Start render loop to check for updates
          const checkRenderLoop = () => {
            if (!mountedRef.current || !chartRef.current) {
              // Clean up animation frame if component unmounted during render loop
              if (animationFrameRef.current) {
                cancelAnimationFrame(animationFrameRef.current);
                animationFrameRef.current = null;
              }
              return;
            }

            // if (chartRef.current.needs_render()) {
            //   chartRef.current.render().catch((err) => {
            //     console.error('[useWasmChart] Render error:', err);
            //   });
            // }

            // Schedule next frame
            animationFrameRef.current = requestAnimationFrame(checkRenderLoop);
          };
          
          // Start the render loop with proper cleanup handling
          animationFrameRef.current = requestAnimationFrame(checkRenderLoop);
        } catch (initError) {
          // Cleanup animation frame on initialization error
          if (animationFrameRef.current) {
            cancelAnimationFrame(animationFrameRef.current);
            animationFrameRef.current = null;
          }
          throw initError;
        }

      } catch (wasmImportError) {
        console.error('[useWasmChart] Failed to import or initialize WASM:', wasmImportError);
        // Cleanup animation frame on WASM import error
        if (animationFrameRef.current) {
          cancelAnimationFrame(animationFrameRef.current);
          animationFrameRef.current = null;
        }
        throw wasmImportError;
      }

      if (!mountedRef.current) return false;


      setChartState(prev => {
        return {
          ...prev,
          chart,
          isInitialized: true
        };
      });

      // Clear loading states on success
      setLoading('wasm', false);
      setLoading('data', false);

      return true;
    } catch (error) {
      console.error('[useWasmChart] Initialize failed:', error);
      
      // Clear loading states on error
      setLoading('wasm', false);
      setLoading('data', false);
      
      return false;
    }
  }, [canvasId, width, height, setLoading]);


  // Performance monitoring interval
  useEffect(() => {
    if (!chartState.chart) return;

    const performanceInterval = setInterval(() => {
      try {
        // Get performance stats from WASM if available
        if (chartState.chart && 'get_performance_stats' in chartState.chart) {
          const stats = (chartState.chart as any).get_performance_stats();
          trackChartPerformance({
            fps: stats.fps || 0,
            renderTime: stats.render_time || 0,
            dataPoints: stats.data_points || 0,
            gpuMemoryUsage: stats.gpu_memory,
            wasmHeapSize: stats.wasm_heap
          });
        }
      } catch (error) {
        console.warn('[useWasmChart] Failed to get performance stats:', error);
      }
    }, 5000); // Every 5 seconds

    return () => clearInterval(performanceInterval);
  }, [chartState.chart, trackChartPerformance]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      mountedRef.current = false;
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
        animationFrameRef.current = null;
      }
    };
  }, []);

  // API object
  const api: WasmChartAPI = {
    initialize,
  };

  return [chartState, api];
}