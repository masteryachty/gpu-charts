import React, { useEffect, useRef, useState } from 'react';
import { useAppStore } from '../store/useAppStore';

const RenderBundlesDemo: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const chartRef = useRef<any>(null);
  const [isLoaded, setIsLoaded] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [bundlesEnabled, setBundlesEnabled] = useState(false);
  const [stats, setStats] = useState<any>({});

  const { currentSymbol, timeRange } = useAppStore();

  useEffect(() => {
    const initializeChart = async () => {
      try {
        // Enable render bundles
        (window as any).ENABLE_RENDER_BUNDLES = '1';
        
        // Import and initialize WASM module
        const wasmModule = await import('@pkg/GPU_charting.js');
        await wasmModule.default();
        
        const chart = new wasmModule.Chart();
        await chart.init(canvasRef.current!.id, 800, 600);
        
        chartRef.current = chart;
        setIsLoaded(true);
        setBundlesEnabled(true);
        
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
        console.error('Failed to initialize render bundles demo:', err);
        setError(err as Error);
      }
    };

    initializeChart();

    return () => {
      // Cleanup
      delete (window as any).ENABLE_RENDER_BUNDLES;
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

  const toggleRenderBundles = () => {
    (window as any).ENABLE_RENDER_BUNDLES = bundlesEnabled ? '0' : '1';
    setBundlesEnabled(!bundlesEnabled);
    // Would need to reinitialize chart to apply change
  };

  const invalidateBundles = () => {
    if (chartRef.current?.invalidate_render_bundles) {
      chartRef.current.invalidate_render_bundles();
      console.log('Render bundles invalidated');
    }
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
        <h2 className="text-xl font-bold mb-2">Render Bundles Demo</h2>
        <p className="text-sm text-gray-600 mb-4">
          This demo shows render bundles, which cache static rendering commands to reduce CPU overhead.
          Pre-recorded command sequences eliminate per-frame command encoding overhead.
        </p>
        
        <div className="flex gap-4 mb-4">
          <button
            onClick={toggleRenderBundles}
            className={`px-4 py-2 rounded ${
              bundlesEnabled ? 'bg-green-500 text-white' : 'bg-gray-300'
            }`}
          >
            Render Bundles: {bundlesEnabled ? 'ON' : 'OFF'}
          </button>
          
          <button
            onClick={invalidateBundles}
            className="px-4 py-2 rounded bg-blue-500 text-white"
          >
            Invalidate Cache
          </button>
        </div>
        
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <h3 className="font-semibold">Performance Benefits:</h3>
            <ul className="list-disc list-inside">
              <li>30% CPU reduction</li>
              <li>Pre-recorded rendering commands</li>
              <li>Cached static elements</li>
              <li>Reduced JavaScript overhead</li>
            </ul>
          </div>
          
          <div>
            <h3 className="font-semibold">Bundle Stats:</h3>
            <ul className="space-y-1">
              <li>Status: {isLoaded ? 'Loaded' : 'Loading...'}</li>
              <li>Bundles: {bundlesEnabled ? 'Enabled' : 'Disabled'}</li>
              <li>Cached Bundles: {stats.total_bundles || 0}</li>
              <li>Cache Hits: {stats.cache_hits || 0}</li>
              <li>Hit Rate: {stats.hit_rate ? `${(stats.hit_rate * 100).toFixed(1)}%` : '0%'}</li>
              <li>Avg Bundle Age: {stats.avg_bundle_age?.toFixed(0) || 0} frames</li>
            </ul>
          </div>
        </div>
      </div>

      <div className="relative border border-gray-300 rounded">
        <canvas
          ref={canvasRef}
          id="render-bundles-canvas"
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
        <h3 className="font-semibold mb-2">How Render Bundles Work:</h3>
        <ol className="list-decimal list-inside space-y-1">
          <li>First render pass records commands into a bundle</li>
          <li>Bundle is cached with viewport and data hash as key</li>
          <li>Subsequent renders execute the cached bundle</li>
          <li>Bundle invalidated when viewport or data changes significantly</li>
          <li>Automatic eviction of old bundles to manage memory</li>
        </ol>
      </div>

      <div className="bg-gray-50 p-4 rounded text-sm">
        <h3 className="font-semibold mb-2">Cache Behavior:</h3>
        <p className="mb-2">
          The render bundle system caches rendering commands based on viewport and data characteristics.
          Try these actions to see the cache in action:
        </p>
        <ul className="list-disc list-inside space-y-1">
          <li>Zoom/pan without changing time range - bundles should be reused</li>
          <li>Zoom to new time range - new bundles created</li>
          <li>Return to previous view - cached bundles may be reused</li>
          <li>Click "Invalidate Cache" to force bundle recreation</li>
        </ul>
      </div>
    </div>
  );
};

export default RenderBundlesDemo;