import { useState, useEffect, useCallback } from 'react';
import { Chart } from '@pkg/wasm_bridge.js';
import { useAppStore } from '../store/useAppStore';
import { arrayBuffer } from 'stream/consumers';


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
  const [metrics, setMetrics] = useState<{ label: string, visible: boolean }[]>([]);

  // Handle preset selection - simplified to use new WASM architecture
  const handlePresetSelect = useCallback(async (preset?: string) => {
    if (preset) {
      try {
        // Apply the preset to the chart instance
        // await chartInstance.apply_preset_and_symbol(
        //   preset,
        //   useAppStore.getState().symbol || ""
        // );
        setPreset(preset);
      } catch (error) {
        console.error('[PresetSection] Error applying preset:', error);
      }
    }

  }, [setPreset]);

  // Load available presets
  useEffect(() => {
    const loadedPresets = chartInstance.get_all_preset_names() || [];
    setPresets(loadedPresets);
  }, [chartInstance]);

  const getMetrics = useCallback(() => {
    if (preset && chartInstance) {
      try {
        const metricsArray = chartInstance.get_metrics_for_preset() || [];
        const metrics = []
        for (let i = 0; i < metricsArray.length; i += 2) {
          metrics.push({ label: metricsArray[i], visible: metricsArray[i + 1] })
        }
        setMetrics(metrics);
      } catch (error) {
        console.error('[PresetSection] Failed to fetch metrics:', error);
      }
    }
  }, [chartInstance, preset]);

  // Load available metrics
  useEffect(() => {
    getMetrics();
  }, [getMetrics]);

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

  // Toggle individual metric visibility - simplified to use new WASM architecture
  const handleMetricToggle = useCallback((chartLabel: string) => {
    // Use the new simplified toggle_metric_visibility method
    chartInstance.toggle_metric_visibility(chartLabel);
    getMetrics()

  }, [chartInstance, getMetrics]);

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

      {/* Chart type checkboxes when preset is active */}
      {preset && metrics.length > 0 && (
        <div className="mt-3 space-y-2">
          <div className="text-xs text-gray-400 mb-1">Chart Components:</div>
          {metrics.map((metric) => (
            <label
              key={metric.label}
              className="flex items-center space-x-2 cursor-pointer hover:bg-gray-700 p-1 rounded transition-colors"
            >
              <input
                type="checkbox"
                checked={metric.visible}
                onChange={() => handleMetricToggle(metric.label)}
                className="w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500 focus:ring-2"
              />
              <span className="text-sm text-gray-300 select-none">
                {metric.label}
              </span>
            </label>
          ))}
        </div>
      )}
    </div>
  );
}