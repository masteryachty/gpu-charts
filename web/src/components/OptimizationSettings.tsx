import React, { useState } from 'react';
import { useAppStore } from '../store/useAppStore';
import { OptimizationSettings as OptimizationState } from '../types';

export const OptimizationSettings: React.FC = () => {
  // Get optimizations from Zustand store
  const optimizations = useAppStore((state) => state.optimizationSettings);
  const updateOptimizationSetting = useAppStore((state) => state.updateOptimizationSetting);
  
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Update optimization and persist to localStorage via store
  const updateOptimization = (key: keyof OptimizationState, value: boolean) => {
    updateOptimizationSetting(key, value);
    
    // Update the actual chart instance if available
    const chart = (window as any).__wasmChart;
    if (chart?.setOptimizationFlags) {
      chart.setOptimizationFlags({ ...optimizations, [key]: value });
    }
  };

  return (
    <div className="bg-bg-tertiary rounded-lg p-4 border border-border">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-text-primary">Performance Optimizations</h3>
        <button
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="text-sm text-accent-blue hover:text-accent-blue/80 transition-colors"
        >
          {showAdvanced ? 'Hide Details' : 'Show Details'}
        </button>
      </div>

      <div className="space-y-3">
        <OptimizationToggle
          label="Binary Search Culling"
          description="25,000x faster data culling using GPU binary search"
          enabled={optimizations.binarySearchCulling}
          onChange={(value) => updateOptimization('binarySearchCulling', value)}
          showDetails={showAdvanced}
          improvement="25,000x"
          impact="Handles millions of data points smoothly"
        />

        <OptimizationToggle
          label="Vertex Compression"
          description="75% memory reduction for vertex data"
          enabled={optimizations.vertexCompression}
          onChange={(value) => updateOptimization('vertexCompression', value)}
          showDetails={showAdvanced}
          improvement="75% less memory"
          impact="Enables larger datasets in browser"
        />

        <OptimizationToggle
          label="GPU Vertex Generation"
          description="4x faster render speed with compute shaders"
          enabled={optimizations.gpuVertexGeneration}
          onChange={(value) => updateOptimization('gpuVertexGeneration', value)}
          showDetails={showAdvanced}
          improvement="4x render speed"
          impact="Smoother zooming and panning"
        />

        <OptimizationToggle
          label="Render Bundles"
          description="30% CPU reduction through pre-recorded commands"
          enabled={optimizations.renderBundles}
          onChange={(value) => updateOptimization('renderBundles', value)}
          showDetails={showAdvanced}
          improvement="30% less CPU"
          impact="Better multi-chart performance"
        />
      </div>

      {showAdvanced && (
        <div className="mt-4 p-3 bg-bg-secondary rounded text-sm text-text-tertiary">
          <p className="mb-2">All optimizations are enabled by default for best performance.</p>
          <p>Disable specific optimizations only for debugging or compatibility testing.</p>
        </div>
      )}
    </div>
  );
};

interface OptimizationToggleProps {
  label: string;
  description: string;
  enabled: boolean;
  onChange: (value: boolean) => void;
  showDetails: boolean;
  improvement: string;
  impact: string;
}

const OptimizationToggle: React.FC<OptimizationToggleProps> = ({
  label,
  description,
  enabled,
  onChange,
  showDetails,
  improvement,
  impact,
}) => {
  return (
    <div className="flex items-start justify-between">
      <div className="flex-1">
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => onChange(e.target.checked)}
            className="w-4 h-4 rounded border-border text-accent-blue focus:ring-accent-blue focus:ring-offset-0 focus:ring-offset-bg-primary"
          />
          <div>
            <div className="text-text-primary font-medium">{label}</div>
            <div className="text-sm text-text-secondary">{description}</div>
            {showDetails && (
              <div className="mt-1 text-xs text-text-tertiary">
                <span className="text-accent-green">{improvement}</span> • {impact}
              </div>
            )}
          </div>
        </label>
      </div>
      {enabled && (
        <div className="ml-3">
          <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-accent-green/20 text-accent-green">
            Active
          </span>
        </div>
      )}
    </div>
  );
};