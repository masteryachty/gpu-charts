import { useRef, useEffect, useCallback } from 'react';
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
  
  // Initialize WASM chart with proper configuration
  const [chartState, chartAPI] = useWasmChart({
    canvasId: 'wasm-chart-canvas',
    width,
    height,
    enableAutoSync,
    debounceMs,
    enableDataFetching: true,
    enablePerformanceMonitoring: showPerformanceOverlay,
  });

  // Initialize chart when canvas is ready
  useEffect(() => {
    if (canvasRef.current && !chartState.isInitialized && !chartState.isLoading) {
      chartAPI.initialize();
    }
  }, [chartState.isInitialized, chartState.isLoading, chartAPI]);

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

  // Mouse move handler
  const handleMouseMove = useCallback((event: React.MouseEvent<HTMLCanvasElement>) => {
    if (chartState.chart && chartState.isInitialized) {
      const rect = canvasRef.current?.getBoundingClientRect();
      if (rect) {
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;
        if (chartState.chart.handle_mouse_move) {
          chartState.chart.handle_mouse_move(x, y);
        }
      }
    }
  }, [chartState.chart, chartState.isInitialized]);

  // Mouse click handler
  const handleMouseClick = useCallback((event: React.MouseEvent<HTMLCanvasElement>) => {
    if (chartState.chart && chartState.isInitialized) {
      const rect = canvasRef.current?.getBoundingClientRect();
      if (rect) {
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;
        if (chartState.chart.handle_mouse_click) {
          chartState.chart.handle_mouse_click(x, y, true);
        }
      }
    }
  }, [chartState.chart, chartState.isInitialized]);

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
        onWheel={handleMouseWheel}
        onMouseMove={handleMouseMove}
        onClick={handleMouseClick}
        data-testid="wasm-canvas"
        data-initialized={chartState.isInitialized ? 'true' : 'false'}
      />
      
      {/* Loading overlay */}
      {chartState.isLoading && (
        <div className="absolute inset-0 bg-gray-900/90 flex items-center justify-center" data-testid="loading-overlay">
          <div className="text-center">
            <div className="animate-spin text-blue-500 text-4xl mb-4">⚡</div>
            <div className="text-white font-medium mb-2">Loading Chart Engine</div>
            <div className="text-gray-400 text-sm">Initializing WebGPU...</div>
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
            {chartState.retryCount < 3 && (
              <button
                onClick={() => chartAPI.retry()}
                className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
              >
                Retry ({chartState.retryCount + 1}/3)
              </button>
            )}
          </div>
        </div>
      )}
      
      {/* Performance overlay */}
      {showPerformanceOverlay && chartState.isInitialized && (
        <div className="absolute top-4 right-4 bg-gray-800/90 text-white text-xs px-3 py-2 rounded backdrop-blur-sm" data-testid="performance-overlay">
          <div>FPS: {chartState.fps}</div>
          <div>Updates: {chartState.updateCount}</div>
          {chartState.renderLatency > 0 && <div>Latency: {chartState.renderLatency}ms</div>}
        </div>
      )}
      
      {/* Debug information */}
      {debugMode && (
        <div className="absolute top-4 left-4 bg-gray-800/90 text-white text-xs px-3 py-2 rounded backdrop-blur-sm max-w-sm" data-testid="debug-overlay">
          <div>Initialized: {chartState.isInitialized ? 'Yes' : 'No'}</div>
          <div>Loading: {chartState.isLoading ? 'Yes' : 'No'}</div>
          <div>Error: {chartState.error ? 'Yes' : 'No'}</div>
          <div>Changes: {chartState.hasUncommittedChanges ? 'Pending' : 'Synced'}</div>
          <div>Last Update: {chartState.lastStateUpdate > 0 ? new Date(chartState.lastStateUpdate).toLocaleTimeString() : 'Never'}</div>
        </div>
      )}
    </div>
  );
}