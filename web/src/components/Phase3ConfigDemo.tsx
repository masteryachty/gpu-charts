import React, { useEffect, useState, useRef } from 'react';

// Import the Phase 3 WASM module
// @ts-ignore - WASM module types will be generated
import init, { ChartSystemMinimal } from '@pkg/gpu_charts_wasm_minimal';

interface ConfigState {
  maxFps: number;
  qualityPreset: string;
  features: {
    scatterPlots: boolean;
    heatmaps: boolean;
    threeDCharts: boolean;
    technicalIndicators: boolean;
  };
  performanceMetrics: {
    fps: number;
    memoryUsageMb: number;
    drawCalls: number;
    vertices: number;
    gpuTimeMs: number;
    cpuTimeMs: number;
  };
}

export const Phase3ConfigDemo: React.FC = () => {
  const [isLoaded, setIsLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [config, setConfig] = useState<ConfigState | null>(null);
  const chartSystemRef = useRef<ChartSystemMinimal | null>(null);
  
  // Component state only - store integration could be added later
  
  useEffect(() => {
    const initializeWasm = async () => {
      try {
        // Initialize the WASM module
        await init();
        
        // Create chart system
        const chartSystem = new ChartSystemMinimal('phase3-demo-canvas');
        chartSystemRef.current = chartSystem;
        
        // Get initial config
        const configJson = chartSystem.get_config();
        const parsedConfig = JSON.parse(configJson);
        
        setConfig({
          maxFps: parsedConfig.max_fps,
          qualityPreset: parsedConfig.quality_preset,
          features: {
            scatterPlots: parsedConfig.scatter_plots,
            heatmaps: parsedConfig.heatmaps,
            threeDCharts: parsedConfig.three_d_charts,
            technicalIndicators: parsedConfig.technical_indicators,
          },
          performanceMetrics: {
            fps: 0,
            memoryUsageMb: 0,
            drawCalls: 0,
            vertices: 0,
            gpuTimeMs: 0,
            cpuTimeMs: 0,
          },
        });
        
        setIsLoaded(true);
      } catch (e) {
        console.error('Failed to initialize Phase 3 WASM:', e);
        setError(e instanceof Error ? e.message : 'Unknown error');
      }
    };
    
    initializeWasm();
    
    return () => {
      // Cleanup
      chartSystemRef.current?.free?.();
    };
  }, []);
  
  // Update performance metrics periodically
  useEffect(() => {
    if (!isLoaded || !chartSystemRef.current) return;
    
    const interval = setInterval(() => {
      const metrics = chartSystemRef.current!.get_performance_metrics();
      const parsed = JSON.parse(metrics);
      
      setConfig(prev => prev ? {
        ...prev,
        performanceMetrics: parsed,
      } : null);
      
      // Log performance metrics (could update store if method existed)
      console.log('Phase 3 Performance:', parsed);
    }, 1000);
    
    return () => clearInterval(interval);
  }, [isLoaded]);
  
  const handleQualityChange = (preset: string) => {
    if (!chartSystemRef.current) return;
    
    try {
      chartSystemRef.current.set_quality_preset(preset);
      
      // Get updated config
      const configJson = chartSystemRef.current.get_config();
      const parsedConfig = JSON.parse(configJson);
      
      setConfig(prev => prev ? {
        ...prev,
        qualityPreset: preset,
        maxFps: parsedConfig.max_fps,
      } : null);
    } catch (e) {
      console.error('Failed to set quality preset:', e);
    }
  };
  
  const handleFpsChange = (fps: number) => {
    if (!chartSystemRef.current) return;
    
    chartSystemRef.current.set_max_fps(fps);
    setConfig(prev => prev ? { ...prev, maxFps: fps } : null);
  };
  
  const handleFeatureToggle = (feature: string) => {
    if (!chartSystemRef.current || !config) return;
    
    // Create updated config
    const updatedConfig = {
      ...JSON.parse(chartSystemRef.current.get_config()),
      [feature]: !config.features[feature as keyof typeof config.features],
    };
    
    try {
      chartSystemRef.current.update_config(JSON.stringify(updatedConfig));
      
      setConfig(prev => prev ? {
        ...prev,
        features: {
          ...prev.features,
          [feature as keyof typeof prev.features]: !prev.features[feature as keyof typeof prev.features],
        },
      } : null);
    } catch (e) {
      console.error('Failed to update config:', e);
    }
  };
  
  const simulateHotReload = () => {
    if (!chartSystemRef.current) return;
    chartSystemRef.current.simulate_hot_reload();
  };
  
  if (error) {
    return (
      <div className="bg-red-900/20 border border-red-500 rounded-lg p-4">
        <h3 className="text-red-400 font-semibold mb-2">Phase 3 Integration Error</h3>
        <p className="text-red-300">{error}</p>
      </div>
    );
  }
  
  if (!isLoaded || !config) {
    return (
      <div className="bg-dark-800 rounded-lg p-6">
        <div className="animate-pulse">
          <div className="h-6 bg-dark-600 rounded w-1/3 mb-4"></div>
          <div className="h-4 bg-dark-600 rounded w-full mb-2"></div>
          <div className="h-4 bg-dark-600 rounded w-3/4"></div>
        </div>
      </div>
    );
  }
  
  return (
    <div className="bg-dark-800 rounded-lg p-6 space-y-6">
      <div>
        <h2 className="text-xl font-semibold mb-4 flex items-center">
          Phase 3 Configuration System
          <span className="ml-2 text-xs bg-green-500/20 text-green-400 px-2 py-1 rounded">
            Active
          </span>
        </h2>
        
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Quality Presets */}
          <div className="space-y-3">
            <h3 className="text-sm font-medium text-gray-400">Quality Preset</h3>
            <div className="flex gap-2">
              {['low', 'medium', 'high', 'ultra'].map(preset => (
                <button
                  key={preset}
                  onClick={() => handleQualityChange(preset)}
                  className={`px-3 py-1 rounded text-sm transition-colors ${
                    config.qualityPreset === preset
                      ? 'bg-blue-500 text-white'
                      : 'bg-dark-700 text-gray-300 hover:bg-dark-600'
                  }`}
                >
                  {preset.charAt(0).toUpperCase() + preset.slice(1)}
                </button>
              ))}
            </div>
          </div>
          
          {/* FPS Limit */}
          <div className="space-y-3">
            <h3 className="text-sm font-medium text-gray-400">FPS Limit: {config.maxFps}</h3>
            <input
              type="range"
              min="30"
              max="240"
              step="30"
              value={config.maxFps}
              onChange={(e) => handleFpsChange(Number(e.target.value))}
              className="w-full"
            />
          </div>
        </div>
      </div>
      
      {/* Features */}
      <div>
        <h3 className="text-sm font-medium text-gray-400 mb-3">Features</h3>
        <div className="grid grid-cols-2 gap-3">
          <label className="flex items-center space-x-2">
            <input
              type="checkbox"
              checked={config.features.scatterPlots}
              onChange={() => handleFeatureToggle('scatter_plots')}
              className="rounded bg-dark-700 border-dark-500"
            />
            <span className="text-sm">Scatter Plots</span>
          </label>
          <label className="flex items-center space-x-2">
            <input
              type="checkbox"
              checked={config.features.heatmaps}
              onChange={() => handleFeatureToggle('heatmaps')}
              className="rounded bg-dark-700 border-dark-500"
            />
            <span className="text-sm">Heatmaps</span>
          </label>
          <label className="flex items-center space-x-2">
            <input
              type="checkbox"
              checked={config.features.threeDCharts}
              onChange={() => handleFeatureToggle('three_d_charts')}
              className="rounded bg-dark-700 border-dark-500"
            />
            <span className="text-sm">3D Charts</span>
          </label>
          <label className="flex items-center space-x-2">
            <input
              type="checkbox"
              checked={config.features.technicalIndicators}
              onChange={() => handleFeatureToggle('technical_indicators')}
              className="rounded bg-dark-700 border-dark-500"
            />
            <span className="text-sm">Technical Indicators</span>
          </label>
        </div>
      </div>
      
      {/* Performance Metrics */}
      <div>
        <h3 className="text-sm font-medium text-gray-400 mb-3">Performance Metrics</h3>
        <div className="grid grid-cols-3 gap-4 text-sm">
          <div>
            <span className="text-gray-500">FPS:</span>
            <span className="ml-2 font-mono">{config.performanceMetrics.fps}</span>
          </div>
          <div>
            <span className="text-gray-500">Memory:</span>
            <span className="ml-2 font-mono">{config.performanceMetrics.memoryUsageMb} MB</span>
          </div>
          <div>
            <span className="text-gray-500">Draw Calls:</span>
            <span className="ml-2 font-mono">{config.performanceMetrics.drawCalls}</span>
          </div>
          <div>
            <span className="text-gray-500">Vertices:</span>
            <span className="ml-2 font-mono">{((config.performanceMetrics.vertices || 0) / 1000000).toFixed(1)}M</span>
          </div>
          <div>
            <span className="text-gray-500">GPU Time:</span>
            <span className="ml-2 font-mono">{(config.performanceMetrics.gpuTimeMs || config.performanceMetrics.gpu_time_ms || 0).toFixed(1)} ms</span>
          </div>
          <div>
            <span className="text-gray-500">CPU Time:</span>
            <span className="ml-2 font-mono">{(config.performanceMetrics.cpuTimeMs || config.performanceMetrics.cpu_time_ms || 0).toFixed(1)} ms</span>
          </div>
        </div>
      </div>
      
      {/* Actions */}
      <div className="flex gap-3">
        <button
          onClick={simulateHotReload}
          className="px-4 py-2 bg-purple-500/20 text-purple-400 rounded hover:bg-purple-500/30 transition-colors"
        >
          Simulate Hot Reload
        </button>
        <button
          onClick={() => {
            const configJson = chartSystemRef.current?.get_config();
            console.log('Current Configuration:', JSON.parse(configJson || '{}'));
          }}
          className="px-4 py-2 bg-gray-500/20 text-gray-400 rounded hover:bg-gray-500/30 transition-colors"
        >
          Log Config
        </button>
      </div>
      
      {/* Hidden canvas for demo */}
      <canvas id="phase3-demo-canvas" style={{ display: 'none' }} />
    </div>
  );
};