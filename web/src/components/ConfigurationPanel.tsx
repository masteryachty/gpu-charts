import React, { useState, useEffect } from 'react';
import { Chart } from '@pkg/GPU_charting';

interface ConfigurationPanelProps {
  chart: Chart | null;
}

interface ChartConfig {
  rendering: {
    target_fps: number;
    line_width: number;
    antialiasing: boolean;
    colors: {
      background: number[];
      grid: number[];
      axis: number[];
      plot: number[];
    };
  };
  performance: {
    max_fps: number;
    chunk_size: number;
    optimize_memory: boolean;
  };
  features: {
    binary_culling: boolean;
    vertex_compression: boolean;
    gpu_vertex_generation: boolean;
    render_bundles: boolean;
  };
}

const ConfigurationPanel: React.FC<ConfigurationPanelProps> = ({ chart }) => {
  const [config, setConfig] = useState<ChartConfig | null>(null);
  const [selectedPreset, setSelectedPreset] = useState<string>('balanced');
  const [isLoading, setIsLoading] = useState(false);

  // Load current configuration when chart is available
  useEffect(() => {
    if (chart) {
      loadConfig();
    }
  }, [chart]);

  const loadConfig = async () => {
    if (!chart) return;
    
    try {
      const configJson = chart.get_config();
      const parsedConfig = JSON.parse(configJson);
      setConfig(parsedConfig);
    } catch (error) {
      console.error('Failed to load configuration:', error);
    }
  };

  const updateConfig = async () => {
    if (!chart || !config) return;
    
    setIsLoading(true);
    try {
      await chart.update_config(JSON.stringify(config));
      console.log('Configuration updated successfully');
    } catch (error) {
      console.error('Failed to update configuration:', error);
    }
    setIsLoading(false);
  };

  const loadPreset = async (preset: string) => {
    if (!chart) return;
    
    setIsLoading(true);
    try {
      await chart.load_config_preset(preset);
      setSelectedPreset(preset);
      // Reload config to reflect changes
      await loadConfig();
    } catch (error) {
      console.error('Failed to load preset:', error);
    }
    setIsLoading(false);
  };

  const toggleFeature = async (feature: string, enabled: boolean) => {
    if (!chart) return;
    
    try {
      await chart.toggle_feature(feature, enabled);
      // Update local state
      if (config) {
        setConfig({
          ...config,
          features: {
            ...config.features,
            [feature]: enabled
          }
        });
      }
    } catch (error) {
      console.error('Failed to toggle feature:', error);
    }
  };

  if (!chart || !config) {
    return (
      <div className="p-4 border rounded-lg bg-gray-50">
        <p className="text-gray-500">Chart not initialized</p>
      </div>
    );
  }

  return (
    <div className="p-4 border rounded-lg bg-white shadow-sm space-y-6">
      <h2 className="text-xl font-semibold mb-4">Chart Configuration</h2>

      {/* Preset Selection */}
      <div>
        <h3 className="text-sm font-medium text-gray-700 mb-2">Configuration Presets</h3>
        <div className="grid grid-cols-2 gap-2">
          {['performance', 'quality', 'balanced', 'low_power'].map((preset) => (
            <button
              key={preset}
              onClick={() => loadPreset(preset)}
              disabled={isLoading}
              className={`px-3 py-2 text-sm rounded transition-colors ${
                selectedPreset === preset
                  ? 'bg-blue-500 text-white'
                  : 'bg-gray-100 hover:bg-gray-200 text-gray-700'
              } ${isLoading ? 'opacity-50 cursor-not-allowed' : ''}`}
            >
              {preset.split('_').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ')}
            </button>
          ))}
        </div>
      </div>

      {/* Feature Flags */}
      <div>
        <h3 className="text-sm font-medium text-gray-700 mb-2">Feature Flags</h3>
        <div className="space-y-2">
          <label className="flex items-center">
            <input
              type="checkbox"
              checked={config.features.binary_culling}
              onChange={(e) => toggleFeature('binary_culling', e.target.checked)}
              className="mr-2"
            />
            <span className="text-sm">Binary Culling</span>
          </label>
          <label className="flex items-center">
            <input
              type="checkbox"
              checked={config.features.vertex_compression}
              onChange={(e) => toggleFeature('vertex_compression', e.target.checked)}
              className="mr-2"
            />
            <span className="text-sm">Vertex Compression</span>
          </label>
          <label className="flex items-center">
            <input
              type="checkbox"
              checked={config.features.gpu_vertex_generation}
              onChange={(e) => toggleFeature('gpu_vertex_generation', e.target.checked)}
              className="mr-2"
            />
            <span className="text-sm">GPU Vertex Generation</span>
          </label>
          <label className="flex items-center">
            <input
              type="checkbox"
              checked={config.features.render_bundles}
              onChange={(e) => toggleFeature('render_bundles', e.target.checked)}
              className="mr-2"
            />
            <span className="text-sm">Render Bundles (Experimental)</span>
          </label>
        </div>
      </div>

      {/* Rendering Settings */}
      <div>
        <h3 className="text-sm font-medium text-gray-700 mb-2">Rendering Settings</h3>
        <div className="space-y-3">
          <div>
            <label className="text-sm text-gray-600">Target FPS</label>
            <input
              type="number"
              value={config.rendering.target_fps}
              onChange={(e) => setConfig({
                ...config,
                rendering: {
                  ...config.rendering,
                  target_fps: parseInt(e.target.value) || 60
                }
              })}
              className="w-full mt-1 px-2 py-1 border rounded text-sm"
              min="10"
              max="144"
            />
          </div>
          <div>
            <label className="text-sm text-gray-600">Line Width</label>
            <input
              type="number"
              value={config.rendering.line_width}
              onChange={(e) => setConfig({
                ...config,
                rendering: {
                  ...config.rendering,
                  line_width: parseFloat(e.target.value) || 2.0
                }
              })}
              className="w-full mt-1 px-2 py-1 border rounded text-sm"
              min="0.5"
              max="10"
              step="0.5"
            />
          </div>
          <label className="flex items-center">
            <input
              type="checkbox"
              checked={config.rendering.antialiasing}
              onChange={(e) => setConfig({
                ...config,
                rendering: {
                  ...config.rendering,
                  antialiasing: e.target.checked
                }
              })}
              className="mr-2"
            />
            <span className="text-sm">Antialiasing</span>
          </label>
        </div>
      </div>

      {/* Performance Settings */}
      <div>
        <h3 className="text-sm font-medium text-gray-700 mb-2">Performance Settings</h3>
        <div className="space-y-3">
          <div>
            <label className="text-sm text-gray-600">Max FPS</label>
            <input
              type="number"
              value={config.performance.max_fps}
              onChange={(e) => setConfig({
                ...config,
                performance: {
                  ...config.performance,
                  max_fps: parseInt(e.target.value) || 60
                }
              })}
              className="w-full mt-1 px-2 py-1 border rounded text-sm"
              min="10"
              max="144"
            />
          </div>
          <label className="flex items-center">
            <input
              type="checkbox"
              checked={config.performance.optimize_memory}
              onChange={(e) => setConfig({
                ...config,
                performance: {
                  ...config.performance,
                  optimize_memory: e.target.checked
                }
              })}
              className="mr-2"
            />
            <span className="text-sm">Optimize Memory</span>
          </label>
        </div>
      </div>

      {/* Apply Button */}
      <button
        onClick={updateConfig}
        disabled={isLoading}
        className={`w-full py-2 px-4 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors ${
          isLoading ? 'opacity-50 cursor-not-allowed' : ''
        }`}
      >
        {isLoading ? 'Applying...' : 'Apply Configuration'}
      </button>
    </div>
  );
};

export default ConfigurationPanel;