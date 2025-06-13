import { useRef, useState } from 'react';

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
  height
}: WasmCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [isLoading] = useState(false);
  const [error] = useState<string | null>(null);

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
      {isLoading && (
        <div className="absolute inset-0 bg-gray-900/90 flex items-center justify-center" data-testid="loading-overlay">
          <div className="text-center">
            <div className="animate-spin text-blue-500 text-4xl mb-4">⚡</div>
            <div className="text-white font-medium mb-2">Loading Chart Engine</div>
            <div className="text-gray-400 text-sm">Initializing WebGPU...</div>
          </div>
        </div>
      )}
      
      {/* Error overlay */}
      {error && (
        <div className="absolute inset-0 bg-gray-900/90 flex items-center justify-center" data-testid="error-overlay">
          <div className="text-center max-w-md">
            <div className="text-red-500 text-4xl mb-4">⚠️</div>
            <div className="text-white font-medium mb-2">Chart Engine Error</div>
            <div className="text-gray-400 text-sm mb-4 break-words">{error}</div>
          </div>
        </div>
      )}
      
      {/* Simplified chart display - with pointer-events: none to allow canvas interactions */}
      <div className="absolute inset-0 flex items-center justify-center pointer-events-none" data-testid="chart-placeholder">
        <div className="text-center">
          <div className="text-white font-medium mb-2">Chart Visualization</div>
          <div className="text-gray-400 text-sm">Simplified testing mode</div>
        </div>
      </div>
    </div>
  );
}