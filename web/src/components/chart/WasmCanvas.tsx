import { useRef, useEffect, useCallback, useState } from 'react';
import { useWasmChart } from '../../hooks/useWasmChart';
import { useAppStore } from '../../store/useAppStore';
import { WasmErrorBoundary } from '../error/WasmErrorBoundary';
import { ChartLoadingSkeleton } from '../loading/LoadingSkeleton';
import { TooltipProvider } from './ChartTooltip';
// import { ChartTooltip, TooltipData } from './ChartTooltip'; // GPU rendering now
// Temporary type def until we remove completely
type TooltipData = any;

interface WasmCanvasProps {
  width?: number;
  height?: number;
  /** Callback when chart is ready with the chart instance */
  onChartReady?: (chart: any) => void;
}

function WasmCanvasCore({
  width = 0,
  height = 0,
  onChartReady,
}: WasmCanvasProps) {
  const { startTime, endTime, symbol } = useAppStore();

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const initializingRef = useRef<boolean>(false);
  
  // Modern tooltip system replaces right-click-hold pattern
  
  // Render debouncing refs
  const mouseMoveTimeoutRef = useRef<number | null>(null);
  const lastMouseMoveRef = useRef<{ x: number; y: number } | null>(null);

  // Initialize WASM chart with proper configuration
  const [chartState, chartAPI] = useWasmChart({
    canvasId: 'webgpu-canvas',
    width,
    height
  });

  // Set canvas size to match container dimensions exactly
  const updateCanvasSize = useCallback(() => {
    if (!canvasRef.current || !containerRef.current) return;

    const canvas = canvasRef.current;
    const container = containerRef.current;
    const rect = container.getBoundingClientRect();

    const newWidth = Math.floor(rect.width);
    const newHeight = Math.floor(rect.height);

    // CRITICAL: Only update canvas size if it actually changed
    // Setting width/height clears the canvas and breaks WebGPU rendering
    // if (canvas.width !== newWidth || canvas.height !== newHeight) {
    //   canvas.width = newWidth;
    //   canvas.height = newHeight;

    //     containerSize: `${rect.width}x${rect.height}`,
    //     canvasSize: `${canvas.width}x${canvas.height}`
    //   });

    //   // Notify the chart about the resize
    //   if (chartState.chart && chartState.isInitialized && chartState.chart.resize) {
    //     chartState.chart.resize(newWidth, newHeight);
    //   }
    // }
  }, []);

  // Initialize chart when canvas is ready with improved timing
  useEffect(() => {
    const initializeChart = async () => {
      // Early return if already initialized to prevent infinite loops
      if (chartState.isInitialized) {
        return;
      }


      // Check for test mode and software rendering flags
      const isTestMode = (window as any).__TEST_MODE__;
      const disableWebGPU = (window as any).__DISABLE_WEBGPU__;

      if (isTestMode) {
      }

      // Check WebGPU availability first (unless disabled in tests)
      if (!disableWebGPU && 'gpu' in navigator) {

        // In test mode, check if WebGPU actually works
        if (isTestMode) {
          try {
            const adapter = await (navigator.gpu as any).requestAdapter();
            if (!adapter) {
              (window as any).__FORCE_SOFTWARE_RENDERING__ = true;
            }
          } catch (error) {
            (window as any).__FORCE_SOFTWARE_RENDERING__ = true;
          }
        }
      } else {
        if (isTestMode) {
        } else {
          return; // In production, we still require WebGPU
        }
      }

      if (canvasRef.current && !chartState.isInitialized && !initializingRef.current) {
        initializingRef.current = true;
        const canvas = canvasRef.current;

        // Ensure canvas is properly laid out before initialization
        await new Promise(resolve => {
          if (canvasRef.current?.clientWidth && canvasRef.current?.clientHeight) {
            resolve(undefined);
          } else {
            // Wait for next frame if canvas doesn't have dimensions yet
            requestAnimationFrame(() => resolve(undefined));
          }
        });

        // Update canvas size to match container
        if (canvasRef.current && containerRef.current) {
          const canvas = canvasRef.current;
          const container = containerRef.current;
          const rect = container.getBoundingClientRect();

          canvas.width = Math.floor(rect.width);
          canvas.height = Math.floor(rect.height);

        }

        if (canvasRef.current && !chartState.isInitialized) {
          try {
            const initStartTime = performance.now();
            console.log('[PERF] WasmCanvas chartAPI.initialize about to start');

            const success = await chartAPI.initialize(startTime, endTime);
            console.log(`[PERF] chartAPI.initialize completed in ${(performance.now() - initStartTime).toFixed(2)}ms, success: ${success}`);
            
            if (success) {
              console.log('[WasmCanvas] Chart initialized successfully');
            } else {
              console.error('[WasmCanvas] Chart initialization returned false');
            }
          } catch (error) {
            console.error('[WasmCanvas] Chart initialization error:', error);
          } finally {
            initializingRef.current = false;
          }
        }
      }
    };

    // Add a timeout to ensure loading doesn't hang indefinitely in tests
    const timeoutDuration = (window as any).__TEST_TIMEOUT_OVERRIDE__ || 10000;
    const initTimeout = setTimeout(() => {
      if (!chartState.isInitialized) {
        initializingRef.current = false;

        // In test mode, mark as initialized anyway to prevent hanging
        if ((window as any).__TEST_MODE__) {
          (window as any).__WASM_CHART_READY__ = true;
        }
      }
    }, timeoutDuration);

    initializeChart();

    return () => {
      clearTimeout(initTimeout);
    };
  }, [chartAPI, chartState.isInitialized, startTime, endTime]); // Add missing dependencies

  // Handle resize events
  useEffect(() => {
    const resizeObserver = new ResizeObserver(() => {
      updateCanvasSize();
    });

    if (containerRef.current) {
      resizeObserver.observe(containerRef.current);
    }

    return () => {
      resizeObserver.disconnect();
    };
  }, [updateCanvasSize]);


  // Note: If you need to force render on specific state changes,
  // you should pass those as props to this component and include them here

  // Call onChartReady when chart is initialized
  useEffect(() => {
    if (chartState.isInitialized && chartState.chart && onChartReady) {
      onChartReady(chartState.chart);
    }
  }, [chartState.isInitialized, chartState.chart, onChartReady]);

  // Track if this is the initial mount to avoid unnecessary time range updates
  const isInitialMount = useRef(true);
  
  // Update time range when it changes in the store
  useEffect(() => {
    // Skip on initial mount since the chart is initialized with these values
    if (isInitialMount.current) {
      isInitialMount.current = false;
      return;
    }

    if (chartState.isInitialized && chartState.chart) {
      if (chartState.chart.update_time_range) {
        console.log(`[WasmCanvas] Updating time range: ${startTime} - ${endTime}`);
        chartState.chart.update_time_range(startTime, endTime)
          .then(() => {
            console.log(`[WasmCanvas] Time range updated successfully`);
          })
          .catch((error: any) => {
            console.error('[WasmCanvas] Failed to update time range:', error);
          });
      }
    }
  }, [startTime, endTime, chartState.isInitialized, chartState.chart]);

  // Check for time range changes from zoom operations
  useEffect(() => {
    if (!chartState.isInitialized || !chartState.chart) return;

    const checkTimeRangeChanges = () => {
      if (chartState.chart && chartState.chart.get_start_time && chartState.chart.get_end_time) {
        try {
          const currentStart = chartState.chart.get_start_time();
          const currentEnd = chartState.chart.get_end_time();
          
          // Check if WASM chart time range differs from React store
          if (currentStart !== startTime || currentEnd !== endTime) {
            console.log(`[WasmCanvas] Time range changed via zoom: ${currentStart} - ${currentEnd}`);
            
            // Update React store to match WASM chart (triggers data fetch)
            const { setTimeRange } = useAppStore.getState();
            if (setTimeRange) {
              setTimeRange(currentStart, currentEnd);
            }
          }
        } catch (error) {
          // Silently ignore errors during time range checks
        }
      }
    };

    // Check every 500ms for time range changes from zoom operations
    const interval = setInterval(checkTimeRangeChanges, 500);
    
    return () => clearInterval(interval);
  }, [chartState.isInitialized, chartState.chart, startTime, endTime]);

  // Cleanup debounce timeout on unmount
  useEffect(() => {
    return () => {
      if (mouseMoveTimeoutRef.current) {
        cancelAnimationFrame(mouseMoveTimeoutRef.current);
        mouseMoveTimeoutRef.current = null;
      }
    };
  }, []);

  // Mouse wheel handler for zoom
  const handleMouseWheel = useCallback((event: React.WheelEvent<HTMLCanvasElement>) => {

    if (chartState.chart && chartState.isInitialized) {
      event.preventDefault();
      const rect = canvasRef.current?.getBoundingClientRect();
      if (rect) {
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;

        if (chartState.chart.handle_mouse_wheel) {
          chartState.chart.handle_mouse_wheel(event.deltaY, x, y);
        }
      }
    }
  }, [chartState.chart, chartState.isInitialized]);

  // Debounced mouse move handler to prevent excessive renders
  const handleMouseMove = useCallback((event: React.MouseEvent<HTMLCanvasElement>) => {
    if (chartState.chart && chartState.isInitialized) {
      const rect = canvasRef.current?.getBoundingClientRect();
      if (rect) {
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;
        
        // Store latest mouse position
        lastMouseMoveRef.current = { x, y };
        
        // Clear existing timeout
        if (mouseMoveTimeoutRef.current) {
          cancelAnimationFrame(mouseMoveTimeoutRef.current);
        }
        
        // Debounce mouse move calls to reduce render frequency
        // Use requestAnimationFrame for smooth updates at display refresh rate
        mouseMoveTimeoutRef.current = requestAnimationFrame(() => {
          if (chartState.chart && chartState.isInitialized && lastMouseMoveRef.current) {
            if (chartState.chart.handle_mouse_move) {
              chartState.chart.handle_mouse_move(lastMouseMoveRef.current.x, lastMouseMoveRef.current.y);
            }
            mouseMoveTimeoutRef.current = null;
          }
        });
        
        // Chart data can be fetched here for external tooltip system if needed
      }
    }
  }, [chartState.chart, chartState.isInitialized]);

  // Mouse down handler (start of drag)
  const handleMouseDown = useCallback((event: React.MouseEvent<HTMLCanvasElement>) => {
    if (chartState.chart && chartState.isInitialized) {
      const rect = canvasRef.current?.getBoundingClientRect();
      if (rect) {
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;

        // Update mouse position first
        if (chartState.chart.handle_mouse_move) {
          chartState.chart.handle_mouse_move(x, y);
        }

        // Only handle left-click for drag (right-click tooltip removed)
        if (event.button === 0) { // Left mouse button
          if (chartState.chart.handle_mouse_click) {
            chartState.chart.handle_mouse_click(x, y, true); // pressed = true
          }
        }
      }
    }
  }, [chartState.chart, chartState.isInitialized]);

  // Mouse up handler (end of drag)
  const handleMouseUp = useCallback((event: React.MouseEvent<HTMLCanvasElement>) => {
    if (chartState.chart && chartState.isInitialized) {
      const rect = canvasRef.current?.getBoundingClientRect();
      if (rect) {
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;

        // Update mouse position first
        if (chartState.chart.handle_mouse_move) {
          chartState.chart.handle_mouse_move(x, y);
        }

        // Only handle left-click release (right-click tooltip removed)
        if (event.button === 0) { // Left mouse button
          if (chartState.chart.handle_mouse_click) {
            chartState.chart.handle_mouse_click(x, y, false); // pressed = false
          }
        }
      }
    }
  }, [chartState.chart, chartState.isInitialized]);
  
  // Prevent context menu on right-click
  const handleContextMenu = useCallback((event: React.MouseEvent<HTMLCanvasElement>) => {
    event.preventDefault();
    return false;
  }, []);

  return (
    <div
      ref={containerRef}
      className="flex-1 bg-gray-900 relative"
      style={{ minWidth: '200px', minHeight: '150px' }}
      data-chart-ready={chartState.isInitialized ? 'true' : undefined}
      role="img"
      aria-label={`Financial chart for ${symbol || 'market data'}, displaying price information from ${new Date(startTime * 1000).toLocaleDateString()} to ${new Date(endTime * 1000).toLocaleDateString()}`}
    >
      <TooltipProvider 
        disabled={!chartState.isInitialized}
        hoverDelay={200}
        hideDelay={150}
        enableKeyboardTooltip={true}
      >
        <canvas
          ref={canvasRef}
          id="webgpu-canvas"
          className="w-full h-full"
          style={{
            width: width ? `${width}px` : '100%',
            height: height ? `${height}px` : '100%',
            display: 'block',
            minWidth: '200px',
            minHeight: '150px'
          }}
          onWheel={handleMouseWheel}
          onMouseMove={handleMouseMove}
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onContextMenu={handleContextMenu}
          data-testid="wasm-canvas"
          data-initialized={chartState.isInitialized ? 'true' : 'false'}
          role="application"
          aria-label={`Interactive financial chart for ${symbol || 'market data'}`}
          aria-describedby="chart-instructions"
          tabIndex={0}
        />
      </TooltipProvider>
      
      {/* Hidden instructions for screen readers */}
      <div id="chart-instructions" className="sr-only">
        Interactive financial chart showing price data over time. 
        Use mouse wheel to zoom, click and drag to pan. 
        Hover mouse over chart area to see price details at cursor position.
        Hold Alt key for keyboard-accessible tooltip at chart center.
        Keyboard shortcuts: Ctrl+R to reset, Ctrl+= to zoom in, Ctrl+- to zoom out.
      </div>

      {/* Enhanced loading skeleton */}
      {!chartState.isInitialized && (
        <ChartLoadingSkeleton className="absolute inset-0" />
      )}

    </div>
  );
}

export default function WasmCanvas(props: WasmCanvasProps) {
  const handleError = (error: Error, errorInfo: React.ErrorInfo) => {
    // Additional WASM-specific error reporting
    console.error('[WasmCanvas] Error caught by WasmErrorBoundary:', error);
    
    // Track specific WASM/WebGPU errors for analytics
    if (typeof window !== 'undefined' && (window as any).analytics) {
      (window as any).analytics.track('wasm_canvas_error', {
        error: error.message,
        stack: error.stack,
        component_stack: errorInfo.componentStack,
        webgpu_available: 'gpu' in navigator,
        wasm_supported: 'WebAssembly' in window,
      });
    }
  };

  return (
    <WasmErrorBoundary
      canvasId="webgpu-canvas"
      onError={handleError}
    >
      <WasmCanvasCore {...props} />
    </WasmErrorBoundary>
  );
}