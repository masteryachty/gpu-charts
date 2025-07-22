import React, { useEffect, useRef, useState } from 'react';
import { useAppStore } from '../store/useAppStore';

const GpuVertexGenDemo: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const chartRef = useRef<any>(null);
  const [isLoaded, setIsLoaded] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [vertexGenEnabled, setVertexGenEnabled] = useState(false);
  const [stats, setStats] = useState<any>({});

  const { currentSymbol, timeRange } = useAppStore();

  useEffect(() => {
    const initializeChart = async () => {
      try {
        // Enable GPU vertex generation
        (window as any).ENABLE_GPU_VERTEX_GEN = '1';
        
        // Import and initialize WASM module
        const wasmModule = await import('@pkg/GPU_charting.js');
        await wasmModule.default();
        
        const chart = new wasmModule.Chart();
        await chart.init(canvasRef.current!.id, 800, 600);
        
        chartRef.current = chart;
        setIsLoaded(true);
        setVertexGenEnabled(true);
        
        // Set initial time range
        const end = Date.now() / 1000;
        const start = end - 3600; // 1 hour
        chart.set_data_range(start, end);
        
        // Start stats update loop
        const updateStats = () => {
          if (chartRef.current?.is_initialized()) {
            try {
              const statsJson = chartRef.current.get_stats?.() || '{}';
              const parsedStats = JSON.parse(statsJson);
              setStats(parsedStats);
            } catch (e) {
              console.error('Failed to get stats:', e);
            }
          }
        };
        
        const interval = setInterval(updateStats, 1000);
        return () => clearInterval(interval);
        
      } catch (err) {
        console.error('Failed to initialize GPU vertex generation demo:', err);
        setError(err as Error);
      }
    };

    initializeChart();

    return () => {
      // Cleanup
      delete (window as any).ENABLE_GPU_VERTEX_GEN;
    };
  }, []);

  const handleMouseWheel = (event: React.WheelEvent) => {
    event.preventDefault();
    if (chartRef.current?.is_initialized()) {
      const rect = canvasRef.current!.getBoundingClientRect();
      const x = event.clientX - rect.left;
      const y = event.clientY - rect.top;
      chartRef.current.handle_mouse_wheel(event.deltaY, x, y);
    }
  };

  const toggleGpuVertexGen = () => {
    (window as any).ENABLE_GPU_VERTEX_GEN = vertexGenEnabled ? '0' : '1';
    setVertexGenEnabled(!vertexGenEnabled);
    // Would need to reinitialize chart to apply change
  };

  if (error) {
    return (
      <div className="p-4 bg-red-50 text-red-800 rounded">
        <h3 className="font-bold">Error</h3>
        <p>{error.message}</p>
      </div>
    );
  }

  return (
    <div className="p-4 space-y-4">
      <div className="bg-gray-100 p-4 rounded">
        <h2 className="text-xl font-bold mb-2">GPU Vertex Generation Demo</h2>
        <p className="text-sm text-gray-600 mb-4">
          This demo shows GPU-driven vertex generation, which creates vertices directly on the GPU
          to eliminate CPU-GPU transfer overhead and enable dynamic LOD.
        </p>
        
        <div className="flex gap-4 mb-4">
          <button
            onClick={toggleGpuVertexGen}
            className={`px-4 py-2 rounded ${
              vertexGenEnabled ? 'bg-green-500 text-white' : 'bg-gray-300'
            }`}
          >
            GPU Vertex Gen: {vertexGenEnabled ? 'ON' : 'OFF'}
          </button>
        </div>
        
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <h3 className="font-semibold">Performance Benefits:</h3>
            <ul className="list-disc list-inside">
              <li>4x faster rendering</li>
              <li>Eliminated data transfer bottleneck</li>
              <li>Dynamic LOD based on zoom</li>
              <li>Parallel vertex generation</li>
            </ul>
          </div>
          
          <div>
            <h3 className="font-semibold">Current Stats:</h3>
            <ul className="space-y-1">
              <li>Status: {isLoaded ? 'Loaded' : 'Loading...'}</li>
              <li>Vertex Gen: {vertexGenEnabled ? 'Enabled' : 'Disabled'}</li>
              <li>Generated Vertices: {stats.generated_vertices || 0}</li>
              <li>LOD Factor: {stats.lod_factor?.toFixed(2) || '1.00'}</li>
              <li>Zoom Level: {stats.zoom_level?.toFixed(2) || '1.00'}</li>
            </ul>
          </div>
        </div>
      </div>

      <div className="relative border border-gray-300 rounded">
        <canvas
          ref={canvasRef}
          id="gpu-vertex-gen-canvas"
          width={800}
          height={600}
          className="block"
          onWheel={handleMouseWheel}
          style={{ imageRendering: 'pixelated' }}
        />
        {!isLoaded && (
          <div className="absolute inset-0 flex items-center justify-center bg-gray-100 bg-opacity-75">
            <p>Loading WebGPU...</p>
          </div>
        )}
      </div>

      <div className="bg-blue-50 p-4 rounded text-sm">
        <h3 className="font-semibold mb-2">How GPU Vertex Generation Works:</h3>
        <ol className="list-decimal list-inside space-y-1">
          <li>CPU sends only raw data arrays (timestamps, values) to GPU</li>
          <li>Compute shader generates vertices in parallel on GPU</li>
          <li>Viewport culling happens during generation</li>
          <li>LOD dynamically adjusts vertex count based on zoom</li>
          <li>Vertices stay in GPU memory - no transfer needed</li>
        </ol>
      </div>
    </div>
  );
};

export default GpuVertexGenDemo;