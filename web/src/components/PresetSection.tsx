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

      {/* Chart type buttons when preset is active */}
      {preset && metrics.length > 0 && (
        <div className="mt-4 space-y-2">
          <label className="text-gray-300 text-sm font-medium">Metrics</label>
          <div className="grid grid-cols-2 gap-2">
            {metrics.map((metric) => {
              const isActive = metric.visible;
              return (
                <button
                  key={metric.label}
                  onClick={() => handleMetricToggle(metric.label)}
                  className={`
                    relative px-3 py-2.5 text-sm font-medium rounded-lg
                    transition-all duration-200 transform
                    ${isActive 
                      ? 'bg-gray-700 text-white shadow-lg scale-[1.02]' 
                      : 'bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200'
                    }
                    border ${isActive ? 'border-gray-500' : 'border-gray-700'}
                    hover:scale-[1.02] active:scale-[0.98]
                  `}
                >
                  <div className="flex items-center justify-center gap-2">
                    {/* Indicator dot */}
                    <div 
                      className={`
                        w-2 h-2 rounded-full transition-all duration-200
                        ${isActive ? 'bg-green-400' : 'bg-gray-600'}
                      `}
                    />
                    <span>{metric.label}</span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}