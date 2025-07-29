import { useState, useEffect, useCallback } from 'react';
import { Chart } from '@pkg/wasm_bridge.js';
import { useAppStore } from '../store/useAppStore';


interface PresetSectionProps {
  chartInstance: Chart; // WASM Chart instance
  preset?: string
}

export default function PresetSection({
  chartInstance,
  preset,
}: PresetSectionProps) {

  const { setPreset } = useAppStore();
  const [presets, setPresets] = useState<string[]>([]);

  // // Handle preset selection - simplified to use new WASM architecture
  const handlePresetSelect = useCallback(async (preset?: string) => {
    if (preset) {
      await chartInstance.apply_preset(
        preset
      );
      setPreset(preset);
    }

  }, [chartInstance, setPreset]);

  // Load available presets
  useEffect(() => {
    const loadPresets = async () => {
      try {
        const loadedPresets = (await chartInstance.get_all_preset_names()) || [];
        setPresets(loadedPresets);
      } catch (err) {
        console.error('[PresetSection] Failed to load presets:', err);
      }
    };

    loadPresets();
  }, [chartInstance, preset]);

  // // Apply Market Data preset by default
  // useEffect(() => {
  //   if (presets.length === 0 || selectedPreset || !chartInstance) return;

  //   // Add a small delay to ensure chart is fully initialized
  //   const timeoutId = setTimeout(() => {
  //     // Find and apply Market Data preset by default
  //     const marketDataPreset = presets.find(p => p.name === 'Market Data');
  //     if (marketDataPreset && chartInstance) {
  //       console.log('[PresetSection] Auto-applying Market Data preset after delay');
  //       handlePresetSelect(marketDataPreset);
  //     }
  //   }, 500); // 500ms delay to ensure chart is ready

  //   return () => clearTimeout(timeoutId);
  // }, [presets, selectedPreset, chartInstance, handlePresetSelect]);

  // // This useEffect is no longer needed since we load chart states immediately in handlePresetSelect

  // // Toggle individual metric visibility - simplified to use new WASM architecture
  // const handleChartTypeToggle = useCallback(async (chartLabel: string) => {
  //   if (!chartInstance?.toggle_metric_visibility) return;

  //   console.log('[PresetSection] Toggling metric visibility:', chartLabel);

  //   try {
  //     // Use the new simplified toggle_metric_visibility method
  //     const toggleResult = chartInstance.toggle_metric_visibility(chartLabel);
  //     const response = JSON.parse(toggleResult);

  //     console.log('[PresetSection] Toggle response:', response);

  //     if (response.success) {
  //       // Get updated visibility states from WASM
  //       const statesJson = chartInstance.get_preset_chart_states();
  //       const statesResponse: PresetChartStatesResponse = JSON.parse(statesJson);

  //       if (statesResponse.success && statesResponse.chart_states) {
  //         setChartStates(statesResponse.chart_states);
  //       }

  //       // Trigger a render to update the display
  //       if (chartInstance.render) {
  //         await chartInstance.render();
  //       }
  //     } else {
  //       throw new Error(response.error || 'Failed to toggle metric visibility');
  //     }
  //   } catch (err) {
  //     console.error('[PresetSection] Error toggling metric visibility:', err);
  //     setError(err instanceof Error ? err.message : 'Failed to toggle metric visibility');
  //   }
  // }, [chartInstance]);

  return (
    <div className="space-y-2">

      {/* Preset selector dropdown */}
      <select
        value={preset || ''}
        onChange={(e) => {
          const presetName = e.target.value;
          handlePresetSelect(presetName);
        }}
        className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm"
      >
        <option value="">Select a Preset</option>
        {presets.map((preset) => (
          <option key={preset} value={preset}>
            {preset}
          </option>
        ))}
      </select>

      {/* Chart type checkboxes when preset is active
      {preset && chartStates.length > 0 && (
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
      )} */}
    </div>
  );
}