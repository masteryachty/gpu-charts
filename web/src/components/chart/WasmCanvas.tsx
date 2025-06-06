import { useEffect, useRef, useState } from 'react';
import { useAppStore } from '../../store/useAppStore';

interface WasmCanvasProps {
  width?: number;
  height?: number;
}

export default function WasmCanvas({ width, height }: WasmCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [simpleChart, setSimpleChart] = useState<any>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  
  const { chartConfig, setConnectionStatus } = useAppStore();

  useEffect(() => {
    let mounted = true;
    let retryCount = 0;
    const maxRetries = 50; // Max 5 seconds of retries

    const loadWasmAndInitChart = async () => {
      try {
        setIsLoading(true);
        setError(null);
        
        // Wait for refs to be available
        if (!canvasRef.current || !containerRef.current) {
          if (retryCount < maxRetries) {
            retryCount++;
            console.log(`Waiting for refs to be available... attempt ${retryCount}`);
            setTimeout(loadWasmAndInitChart, 100);
            return;
          } else {
            throw new Error('Canvas or container ref never became available');
          }
        }
        
        // Import the WASM module using the Vite alias
        const wasmModule = await import('@pkg/tutorial1_window.js');
        
        if (!mounted) return;
        
        // Initialize the WASM module
        await wasmModule.default();
        
        if (!mounted) return;

        const canvas = canvasRef.current;
        const container = containerRef.current;
        
        // Verify canvas is in the DOM with correct ID
        const domCanvas = document.getElementById('new-api-canvas');
        if (!domCanvas) {
          throw new Error('Canvas with id "new-api-canvas" not found in DOM');
        }
        
        // Set up canvas dimensions
        const actualWidth = width || container.clientWidth || 800;
        const actualHeight = height || container.clientHeight || 600;
        
        canvas.width = actualWidth;
        canvas.height = actualHeight;
        
        console.log(`Canvas setup: ${actualWidth}x${actualHeight}, DOM element:`, domCanvas);
        
        // Add a small delay to ensure DOM is fully ready
        await new Promise(resolve => setTimeout(resolve, 100));
        
        // Create SimpleChart instance
        const chart = new wasmModule.SimpleChart();
        
        console.log('About to call chart.init with canvas ID:', canvas.id);
        
        // Initialize the chart with the canvas
        chart.init(canvas.id);
        
        if (!mounted) return;
        
        setSimpleChart(chart);
        setConnectionStatus(true);
        
        console.log('Chart initialized successfully with config:', chartConfig);
        
      } catch (err) {
        console.error('Failed to load WASM module or initialize chart:', err);
        if (mounted) {
          setError(`Failed to load chart engine: ${err}`);
          setConnectionStatus(false);
        }
      } finally {
        if (mounted) {
          setIsLoading(false);
        }
      }
    };

    loadWasmAndInitChart();

    return () => {
      mounted = false;
    };
  }, [width, height, setConnectionStatus, chartConfig]);

  // Handle chart config changes
  useEffect(() => {
    if (simpleChart && simpleChart.is_initialized()) {
      // Chart is ready for updates
      console.log('Chart ready for config updates:', chartConfig);
      // Future: call chart update methods here
    }
  }, [chartConfig, simpleChart]);

  // Handle canvas resize
  useEffect(() => {
    const handleResize = () => {
      if (canvasRef.current && containerRef.current) {
        const canvas = canvasRef.current;
        const container = containerRef.current;
        
        const newWidth = width || container.clientWidth;
        const newHeight = height || container.clientHeight;
        
        if (canvas.width !== newWidth || canvas.height !== newHeight) {
          canvas.width = newWidth;
          canvas.height = newHeight;
          
          // Notify WASM about resize
          if (simpleChart) {
            console.log('Canvas resized:', newWidth, newHeight);
            // Future: call simpleChart.resize(newWidth, newHeight);
          }
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
  }, [simpleChart, width, height]);

  return (
    <div 
      ref={containerRef}
      className="flex-1 bg-bg-secondary border border-border relative overflow-hidden"
    >
      <canvas
        ref={canvasRef}
        id="new-api-canvas"
        className="w-full h-full"
        style={{ 
          width: '100%', 
          height: '100%',
          display: 'block'
        }}
      />
      
      {/* Loading overlay */}
      {isLoading && (
        <div className="absolute inset-0 bg-bg-secondary/90 flex items-center justify-center">
          <div className="text-center">
            <div className="animate-spin text-accent-blue text-4xl mb-4">⚡</div>
            <div className="text-text-primary font-medium mb-2">Loading Chart Engine</div>
            <div className="text-text-secondary text-sm">Initializing WebGPU...</div>
          </div>
        </div>
      )}
      
      {/* Error overlay */}
      {error && (
        <div className="absolute inset-0 bg-bg-secondary/90 flex items-center justify-center">
          <div className="text-center">
            <div className="text-accent-red text-4xl mb-4">⚠️</div>
            <div className="text-text-primary font-medium mb-2">Chart Engine Error</div>
            <div className="text-text-secondary text-sm">{error}</div>
          </div>
        </div>
      )}
      
      {/* Performance overlay */}
      {!isLoading && !error && (
        <div className="absolute top-4 right-4 bg-bg-tertiary/80 border border-border px-3 py-2 text-xs font-mono">
          <div className="text-accent-green">120 FPS</div>
          <div className="text-text-secondary">8.33ms</div>
        </div>
      )}
    </div>
  );
}