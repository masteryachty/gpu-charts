import { useState, useEffect, useCallback } from 'react';
import type { 
  RenderingPreset, 
  PresetListResponse, 
  ChartStateInfo,
  PresetChartStatesResponse,
  ToggleChartTypeResponse 
} from '../types';

interface PresetSectionProps {
  chartInstance: any; // WASM Chart instance
  currentSymbol: string;
  startTime: number;
  endTime: number;
  onPresetChange?: (presetName: string | null) => void;
}

export default function PresetSection({ 
  chartInstance, 
  currentSymbol,
  startTime,
  endTime,
  onPresetChange 
}: PresetSectionProps) {
  const [presets, setPresets] = useState<RenderingPreset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState<RenderingPreset | null>(null);
  const [chartStates, setChartStates] = useState<ChartStateInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Handle preset selection - simplified to use new WASM architecture
  const handlePresetSelect = useCallback(async (preset: RenderingPreset | null) => {
    if (!chartInstance?.apply_preset || !chartInstance?.clear_preset) {
      return;
    }

    setError(null);
    setIsLoading(true);

    try {
      if (preset) {
        // Apply preset with auto data fetching
        // WASM apply_preset() now fetches data automatically
        const applyResult = await chartInstance.apply_preset(
          preset.name,
          currentSymbol,
          BigInt(startTime),
          BigInt(endTime)
        );
        const applyResponse = JSON.parse(applyResult);

        if (!applyResponse.success) {
          throw new Error(applyResponse.message || 'Failed to apply preset');
        }

        // Update state
        setSelectedPreset(preset);
        onPresetChange?.(preset.name);

        // Load chart states to show checkboxes
        try {
          const statesJson = chartInstance.get_preset_chart_states();
          const response: PresetChartStatesResponse = JSON.parse(statesJson);
          
          if (response.success && response.chart_states) {
            setChartStates(response.chart_states);
          }
        } catch (err) {
          console.error('[PresetSection] Failed to get chart states:', err);
        }

        // Trigger a render to display the data
        if (chartInstance.render) {
          await chartInstance.render();
        }
      } else {
        // Clear preset
        const clearResult = chartInstance.clear_preset();
        const clearResponse = JSON.parse(clearResult);
        
        if (!clearResponse.success) {
          throw new Error(clearResponse.message || 'Failed to clear preset');
        }
        
        setSelectedPreset(null);
        setChartStates([]);
        onPresetChange?.(null);
        
        // Trigger a render to clear the display
        if (chartInstance.render) {
          await chartInstance.render();
        }
      }
    } catch (err) {
      console.error('[PresetSection] Error applying preset:', err);
      setError(err instanceof Error ? err.message : 'Failed to apply preset');
    } finally {
      setIsLoading(false);
    }
  }, [chartInstance, currentSymbol, startTime, endTime, onPresetChange]);

  // Load available presets
  useEffect(() => {
    if (!chartInstance?.list_presets) return;

    try {
      const presetsJson = chartInstance.list_presets();
      const response: PresetListResponse = JSON.parse(presetsJson);
      const loadedPresets = response.presets || [];
      setPresets(loadedPresets);
    } catch (err) {
      console.error('[PresetSection] Failed to load presets:', err);
      setError('Failed to load presets');
    }
  }, [chartInstance]);

  // Apply Market Data preset by default
  useEffect(() => {
    if (presets.length === 0 || selectedPreset || !chartInstance) return;
    
    // Add a small delay to ensure chart is fully initialized
    const timeoutId = setTimeout(() => {
      // Find and apply Market Data preset by default
      const marketDataPreset = presets.find(p => p.name === 'Market Data');
      if (marketDataPreset && chartInstance) {
        console.log('[PresetSection] Auto-applying Market Data preset after delay');
        handlePresetSelect(marketDataPreset);
      }
    }, 500); // 500ms delay to ensure chart is ready
    
    return () => clearTimeout(timeoutId);
  }, [presets, selectedPreset, chartInstance, handlePresetSelect]);

  // This useEffect is no longer needed since we load chart states immediately in handlePresetSelect

  // Toggle individual metric visibility - simplified to use new WASM architecture
  const handleChartTypeToggle = useCallback(async (chartLabel: string) => {
    if (!chartInstance?.toggle_metric_visibility) return;

    console.log('[PresetSection] Toggling metric visibility:', chartLabel);

    try {
      // Use the new simplified toggle_metric_visibility method
      const toggleResult = chartInstance.toggle_metric_visibility(chartLabel);
      const response = JSON.parse(toggleResult);
      
      console.log('[PresetSection] Toggle response:', response);
      
      if (response.success) {
        // Get updated visibility states from WASM
        const statesJson = chartInstance.get_preset_chart_states();
        const statesResponse: PresetChartStatesResponse = JSON.parse(statesJson);
        
        if (statesResponse.success && statesResponse.chart_states) {
          setChartStates(statesResponse.chart_states);
        }
        
        // Trigger a render to update the display
        if (chartInstance.render) {
          await chartInstance.render();
        }
      } else {
        throw new Error(response.error || 'Failed to toggle metric visibility');
      }
    } catch (err) {
      console.error('[PresetSection] Error toggling metric visibility:', err);
      setError(err instanceof Error ? err.message : 'Failed to toggle metric visibility');
    }
  }, [chartInstance]);

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <label className="text-gray-300 text-sm font-medium">Preset</label>
        {isLoading && (
          <div className="flex items-center text-xs text-gray-400">
            <div className="animate-spin rounded-full h-3 w-3 border-b-2 border-white mr-1"></div>
            Loading data...
          </div>
        )}
      </div>
      
      {/* Preset selector dropdown */}
      <select
        value={selectedPreset?.name || ''}
        onChange={(e) => {
          const presetName = e.target.value;
          if (!presetName) {
            handlePresetSelect(null);
          } else {
            const preset = presets.find(p => p.name === presetName);
            if (preset) {
              handlePresetSelect(preset);
            }
          }
        }}
        className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm"
      >
        <option value="">Select a Preset</option>
        {presets.map((preset) => (
          <option key={preset.name} value={preset.name}>
            {preset.name}
          </option>
        ))}
      </select>

      {/* Chart type checkboxes when preset is active */}
      {selectedPreset && chartStates.length > 0 && (
        <div className="mt-3 space-y-2">
          <div className="text-xs text-gray-400 mb-1">Chart Components:</div>
          {chartStates.map((chartState) => (
            <label 
              key={chartState.label}
              className="flex items-center space-x-2 cursor-pointer hover:bg-gray-700 p-1 rounded transition-colors"
            >
              <input
                type="checkbox"
                checked={chartState.visible}
                onChange={() => handleChartTypeToggle(chartState.label)}
                className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500 focus:ring-2"
              />
              <span className="text-sm text-gray-300 select-none">
                {chartState.label}
              </span>
            </label>
          ))}
        </div>
      )}


      {/* Error display */}
      {error && (
        <div className="text-xs text-red-400 mt-1">
          {error}
        </div>
      )}

      {/* Current preset info */}
      {selectedPreset && !isLoading && (
        <div className="mt-2 text-xs text-gray-500">
          {selectedPreset.description}
        </div>
      )}
    </div>
  );
}