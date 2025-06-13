import { useCallback, useEffect, useRef, useState } from 'react';
import { useAppStore } from '../../store/useAppStore';
import { useWasmChart } from '../../hooks/useWasmChart';

interface WasmCanvasProps {
  width?: number;
  height?: number;
  
  /** Enable automatic store synchronization (default: true) */
  enableAutoSync?: boolean;
  
  /** Debounce delay for state changes in ms (default: 100) */
  debounceMs?: number;
  
  /** Enable performance monitoring overlay (default: true) */
  showPerformanceOverlay?: boolean;
  
  /** Enable debug information (default: false) */
  debugMode?: boolean;
}

export default function WasmCanvas({ 
  width, 
  height, 
  enableAutoSync = true,
  debounceMs = 100,
  showPerformanceOverlay = true,
  debugMode = false
}: WasmCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [actualDimensions, setActualDimensions] = useState<{width: number, height: number} | null>(null);
  
  const { setConnectionStatus } = useAppStore();

  // Use the advanced WASM chart hook
  const [chartState, chartAPI] = useWasmChart({
    canvasId: 'wasm-chart-canvas',
    width: actualDimensions?.width || width,
    height: actualDimensions?.height || height,
    enableAutoSync,
    enableDataFetching: true,
    debounceMs,
    enablePerformanceMonitoring: showPerformanceOverlay,
    maxRetries: 3,
    retryDelayMs: 1000,
  });

  // Update connection status when chart state changes
  useEffect(() => {
    setConnectionStatus(chartState.isInitialized && !chartState.error);
  }, [chartState.isInitialized, chartState.error]);

  // Initialize chart when canvas is ready
  useEffect(() => {
    const initializeWhenReady = async () => {
      // Wait for canvas and container to be available
      if (!canvasRef.current || !containerRef.current) {
        return;
      }

      // Calculate actual dimensions
      const container = containerRef.current;
      const actualWidth = width || container.clientWidth || 800;
      const actualHeight = height || container.clientHeight || 600;

      // Update dimensions state
      setActualDimensions({ width: actualWidth, height: actualHeight });

      // Set canvas dimensions
      const canvas = canvasRef.current;
      canvas.width = actualWidth;
      canvas.height = actualHeight;

      // Initialize the chart
      console.log('[WasmCanvas] Chart state check:', { 
        isInitialized: chartState.isInitialized, 
        isLoading: chartState.isLoading,
        shouldInitialize: !chartState.isInitialized && !chartState.isLoading 
      });
      
      if (!chartState.isInitialized && !chartState.isLoading) {
        console.log('[WasmCanvas] Initializing chart with dimensions:', { actualWidth, actualHeight });
        await chartAPI.initialize();
      } else {
        console.log('[WasmCanvas] Skipping initialization:', { 
          reason: chartState.isInitialized ? 'already initialized' : 'currently loading' 
        });
      }
    };

    // Delay to ensure DOM is ready
    const timeout = setTimeout(initializeWhenReady, 50);
    return () => clearTimeout(timeout);
  }, [width, height, chartState.isInitialized, chartState.isLoading, chartAPI]);

  // Handle canvas resize with optimized WASM notification
  useEffect(() => {
    const handleResize = async () => {
      if (!canvasRef.current || !containerRef.current) return;

      const canvas = canvasRef.current;
      const container = containerRef.current;
      
      const newWidth = width || container.clientWidth;
      const newHeight = height || container.clientHeight;
      
      // Only update if dimensions actually changed
      if (actualDimensions && 
          (newWidth !== actualDimensions.width || newHeight !== actualDimensions.height)) {
        
        console.log('[WasmCanvas] Resizing canvas:', { 
          from: actualDimensions, 
          to: { width: newWidth, height: newHeight } 
        });

        // Update canvas dimensions
        canvas.width = newWidth;
        canvas.height = newHeight;
        
        // Update state
        setActualDimensions({ width: newWidth, height: newHeight });
        
        // Note: SimpleChart doesn't support resize notifications
        if (chartState.chart && chartState.isInitialized) {
          console.log('[WasmCanvas] Canvas resized (SimpleChart mode):', { newWidth, newHeight });
          // SimpleChart doesn't have a resize method, so we just log it
        }
      }
    };

    const resizeObserver = new ResizeObserver(handleResize);
    if (containerRef.current) {
      resizeObserver.observe(containerRef.current);
    }

    return () => {
      resizeObserver.disconnect();
    };
  }, [width, height, actualDimensions, chartState.chart, chartState.isInitialized]);

  // Manual retry handler
  const handleRetry = useCallback(async () => {
    console.log('[WasmCanvas] Manual retry triggered');
    await chartAPI.retry();
  }, [chartAPI]);

  // Manual reset handler
  const handleReset = useCallback(async () => {
    console.log('[WasmCanvas] Manual reset triggered');
    await chartAPI.reset();
  }, [chartAPI]);

  // Debug panel actions
  const handleForceUpdate = useCallback(async () => {
    console.log('[WasmCanvas] Force update triggered');
    await chartAPI.forceUpdate();
  }, [chartAPI]);

  const handleGetCurrentState = useCallback(async () => {
    const state = await chartAPI.getCurrentState();
    console.log('[WasmCanvas] Current WASM state:', state);
  }, [chartAPI]);

  return (
    <div 
      ref={containerRef}
      className="flex-1 bg-gray-900 border border-gray-700 relative overflow-hidden"
    >
      <canvas
        ref={canvasRef}
        id="wasm-chart-canvas"
        className="w-full h-full"
        style={{ 
          width: '100%', 
          height: '100%',
          display: 'block'
        }}
      />
      
      {/* Loading overlay */}
      {chartState.isLoading && (
        <div className="absolute inset-0 bg-gray-900/90 flex items-center justify-center" data-testid="loading-overlay">
          <div className="text-center">
            <div className="animate-spin text-blue-500 text-4xl mb-4">⚡</div>
            <div className="text-white font-medium mb-2">Loading Chart Engine</div>
            <div className="text-gray-400 text-sm">
              {chartState.retryCount > 0 
                ? `Initializing WebGPU... (Retry ${chartState.retryCount})`
                : 'Initializing WebGPU...'
              }
            </div>
            {chartState.hasUncommittedChanges && (
              <div className="text-yellow-500 text-xs mt-2">Syncing store state...</div>
            )}
          </div>
        </div>
      )}
      
      {/* Error overlay */}
      {chartState.error && (
        <div className="absolute inset-0 bg-gray-900/90 flex items-center justify-center" data-testid="error-overlay">
          <div className="text-center max-w-md">
            <div className="text-red-500 text-4xl mb-4">⚠️</div>
            <div className="text-white font-medium mb-2">Chart Engine Error</div>
            <div className="text-gray-400 text-sm mb-4 break-words">{chartState.error}</div>
            
            <div className="flex gap-2 justify-center">
              {chartState.retryCount < 3 && (
                <button
                  onClick={handleRetry}
                  className="px-4 py-2 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 transition-colors"
                  data-testid="retry-button"
                >
                  Retry ({3 - chartState.retryCount} left)
                </button>
              )}
              
              <button
                onClick={handleReset}
                className="px-4 py-2 bg-gray-600 text-white text-sm rounded hover:bg-gray-700 transition-colors"
                data-testid="reset-button"
              >
                Reset
              </button>
            </div>
            
            {chartState.lastError && chartState.lastError !== chartState.error && (
              <div className="mt-4 text-gray-500 text-xs">
                Previous: {chartState.lastError}
              </div>
            )}
          </div>
        </div>
      )}
      
      {/* Performance overlay */}
      {showPerformanceOverlay && chartState.isInitialized && !chartState.error && (
        <div className="absolute top-4 right-4 bg-gray-800/80 border border-gray-600 px-3 py-2 text-xs font-mono">
          <div className="text-green-500">{chartState.fps || 0} FPS</div>
          <div className="text-gray-400">{chartState.renderLatency.toFixed(1)}ms</div>
          <div className="text-blue-400">#{chartState.updateCount}</div>
          {chartState.hasUncommittedChanges && (
            <div className="text-yellow-500">SYNC</div>
          )}
          {chartState.dataFetchingEnabled && chartState.lastDataFetch && (
            <div className="border-t border-gray-600 mt-1 pt-1">
              <div className="text-purple-400">
                {chartState.lastDataFetch.recordCount.toLocaleString()} records
              </div>
              <div className="text-gray-400">
                {chartState.lastDataFetch.fromCache ? 'cached' : 'fetched'}
              </div>
            </div>
          )}
        </div>
      )}
      
      {/* Debug panel */}
      {debugMode && chartState.isInitialized && (
        <div className="absolute bottom-4 left-4 bg-gray-800/90 border border-gray-600 p-3 text-xs font-mono space-y-2">
          <div className="text-white font-bold mb-2">Debug Panel</div>
          
          <div className="space-y-1">
            <div className="text-gray-400">
              State: <span className="text-white">{chartState.isInitialized ? 'Ready' : 'Not Ready'}</span>
            </div>
            <div className="text-gray-400">
              Updates: <span className="text-white">{chartState.updateCount}</span>
            </div>
            <div className="text-gray-400">
              Last Update: <span className="text-white">
                {chartState.lastStateUpdate > 0 
                  ? new Date(chartState.lastStateUpdate).toLocaleTimeString()
                  : 'Never'
                }
              </span>
            </div>
            <div className="text-gray-400">
              Auto Sync: <span className={enableAutoSync ? 'text-green-500' : 'text-red-500'}>
                {enableAutoSync ? 'ON' : 'OFF'}
              </span>
            </div>
          </div>
          
          <div className="flex gap-1 pt-2">
            <button
              onClick={handleForceUpdate}
              className="px-2 py-1 bg-blue-600 text-white text-xs rounded hover:bg-blue-700"
              data-testid="force-update-button"
            >
              Force Update
            </button>
            
            <button
              onClick={handleGetCurrentState}
              className="px-2 py-1 bg-green-600 text-white text-xs rounded hover:bg-green-700"
              data-testid="get-state-button"
            >
              Log State
            </button>
          </div>
        </div>
      )}
      
      {/* Store sync indicator */}
      {enableAutoSync && chartState.isInitialized && (
        <div className={`absolute top-4 left-4 w-3 h-3 rounded-full ${
          chartState.hasUncommittedChanges 
            ? 'bg-yellow-500 animate-pulse' 
            : 'bg-green-500'
        }`} title={chartState.hasUncommittedChanges ? 'Syncing...' : 'Synced'} />
      )}
    </div>
  );
}