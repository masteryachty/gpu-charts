import { useState, useEffect, useCallback, useRef } from 'react';
import { useAppStore } from '../store/useAppStore';
import type { PresetGroup, RenderingPreset, PresetListResponse } from '../types';

interface PresetSelectorProps {
  chartInstance: any; // WASM Chart instance
  onPresetChange?: (presetName: string | null) => void;
  className?: string;
}

export default function PresetSelector({ 
  chartInstance, 
  onPresetChange,
  className = ''
}: PresetSelectorProps) {
  const [presetGroups, setPresetGroups] = useState<PresetGroup[]>([]);
  const [selectedPreset, setSelectedPreset] = useState<RenderingPreset | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isFetchingData, setIsFetchingData] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Get store state and actions
  const { currentSymbol, chartConfig, updateChartState } = useAppStore();

  // Load available presets
  useEffect(() => {
    if (!chartInstance) {
      console.log('[PresetSelector] Chart instance not yet available');
      return;
    }

    console.log('[PresetSelector] Chart instance:', chartInstance);
    console.log('[PresetSelector] Chart methods:', Object.getOwnPropertyNames(Object.getPrototypeOf(chartInstance)));

    if (!chartInstance.list_presets) {
      console.warn('[PresetSelector] Chart instance does not have list_presets method');
      return;
    }

    try {
      const presetsJson = chartInstance.list_presets();
      console.log('[PresetSelector] Raw presets JSON:', presetsJson);
      const response: PresetListResponse = JSON.parse(presetsJson);
      console.log('[PresetSelector] Parsed response:', response);
      setPresetGroups(response.groups || []);
    } catch (err) {
      console.error('[PresetSelector] Failed to load presets:', err);
      setError('Failed to load presets');
    }
  }, [chartInstance]);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Apply selected preset
  const handlePresetSelect = useCallback(async (preset: RenderingPreset | null) => {
    if (!chartInstance?.apply_preset || !chartInstance?.fetch_preset_data || !chartInstance?.clear_preset) {
      console.warn('[PresetSelector] Chart instance missing preset methods');
      return;
    }

    setIsLoading(true);
    setError(null);
    setIsOpen(false);

    try {
      if (preset) {
        // Apply the preset configuration
        const applyResult = chartInstance.apply_preset(preset.name);
        const applyResponse = JSON.parse(applyResult);

        if (!applyResponse.success) {
          throw new Error(applyResponse.message || 'Failed to apply preset');
        }

        // Fetch data for the preset
        setIsFetchingData(true);
        const dataResult = await chartInstance.fetch_preset_data(
          currentSymbol,
          BigInt(chartConfig.startTime),
          BigInt(chartConfig.endTime)
        );
        
        const dataResponse = JSON.parse(dataResult);
        if (!dataResponse.success) {
          throw new Error(dataResponse.error || 'Failed to fetch preset data');
        }

        // Update store with preset-specific configuration
        // This might include specific metrics, chart types, etc.
        // The exact updates depend on the preset configuration
        updateChartState({
          // The preset manager will have configured the chart appropriately
          // We may need to update the store to reflect these changes
        });

        setSelectedPreset(preset);
        onPresetChange?.(preset.name);
      } else {
        // Clear preset - return to manual mode
        const clearResult = chartInstance.clear_preset();
        const clearResponse = JSON.parse(clearResult);
        
        if (!clearResponse.success) {
          throw new Error(clearResponse.message || 'Failed to clear preset');
        }
        
        setSelectedPreset(null);
        onPresetChange?.(null);
      }
    } catch (err) {
      console.error('[PresetSelector] Error applying preset:', err);
      setError(err instanceof Error ? err.message : 'Failed to apply preset');
    } finally {
      setIsLoading(false);
      setIsFetchingData(false);
    }
  }, [chartInstance, currentSymbol, chartConfig.startTime, chartConfig.endTime, updateChartState, onPresetChange]);

  // Format preset for display
  const formatPresetDisplay = (preset: RenderingPreset | null) => {
    if (!preset) return 'Select a preset...';
    
    // Find which group this preset belongs to
    const group = presetGroups.find(g => 
      g.presets.some(p => p.name === preset.name)
    );
    
    return group ? `${group.name} - ${preset.name}` : preset.name;
  };

  return (
    <div className={`relative ${className}`} ref={dropdownRef}>
      {/* Dropdown button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        disabled={isLoading || isFetchingData}
        className="relative w-full cursor-pointer rounded-lg bg-gray-700 border border-gray-600 py-2 pl-3 pr-10 text-left text-white shadow-md hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-gray-900 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        <span className="block truncate">
          {isLoading ? 'Applying preset...' : 
           isFetchingData ? 'Fetching data...' :
           formatPresetDisplay(selectedPreset)}
        </span>
        <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
          <svg className="h-5 w-5 text-gray-400" viewBox="0 0 20 20" fill="currentColor">
            <path fillRule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clipRule="evenodd" />
          </svg>
        </span>
      </button>

      {/* Dropdown menu */}
      {isOpen && (
        <div className="absolute z-50 mt-1 w-full rounded-md bg-gray-800 py-1 shadow-lg ring-1 ring-black ring-opacity-5 max-h-60 overflow-auto">
          {/* Clear preset option */}
          <button
            onClick={() => handlePresetSelect(null)}
            className={`relative w-full cursor-pointer select-none py-2 pl-10 pr-4 text-left hover:bg-gray-700 hover:text-white transition-colors ${
              selectedPreset === null ? 'bg-gray-700 text-white' : 'text-gray-300'
            }`}
          >
            <span className={`block truncate ${selectedPreset === null ? 'font-medium' : 'font-normal'}`}>
              Manual Mode (No Preset)
            </span>
            {selectedPreset === null && (
              <span className="absolute inset-y-0 left-0 flex items-center pl-3 text-blue-400">
                <svg className="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                  <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                </svg>
              </span>
            )}
          </button>

          {/* Preset groups */}
          {(presetGroups || []).map((group) => (
            <div key={group.name}>
              <div className="px-4 py-2 text-xs font-semibold text-gray-400 uppercase tracking-wider">
                {group.name}
              </div>
              {group.presets.map((preset) => (
                <button
                  key={preset.name}
                  onClick={() => handlePresetSelect(preset)}
                  className={`relative w-full cursor-pointer select-none py-2 pl-10 pr-4 text-left hover:bg-gray-700 hover:text-white transition-colors ${
                    selectedPreset?.name === preset.name ? 'bg-gray-700 text-white' : 'text-gray-300'
                  }`}
                  title={preset.description}
                >
                  <span className={`block truncate ${selectedPreset?.name === preset.name ? 'font-medium' : 'font-normal'}`}>
                    {preset.name}
                  </span>
                  <span className="block text-xs text-gray-500 truncate">
                    {preset.description}
                  </span>
                  {selectedPreset?.name === preset.name && (
                    <span className="absolute inset-y-0 left-0 flex items-center pl-3 text-blue-400">
                      <svg className="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                        <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                      </svg>
                    </span>
                  )}
                </button>
              ))}
            </div>
          ))}
        </div>
      )}

      {/* Error display */}
      {error && (
        <div className="mt-2 text-sm text-red-400">
          {error}
        </div>
      )}

      {/* Loading indicator overlay */}
      {(isLoading || isFetchingData) && (
        <div className="absolute inset-0 flex items-center justify-center bg-gray-900 bg-opacity-50 rounded-lg">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-white"></div>
        </div>
      )}

      {/* Current preset info */}
      {selectedPreset && !isLoading && !isFetchingData && (
        <div className="mt-2 p-2 bg-gray-800 rounded text-xs text-gray-400">
          <div className="font-medium text-gray-300">Active Preset: {selectedPreset.name}</div>
          <div className="mt-1">{selectedPreset.description}</div>
          <div className="mt-1">
            Charts: {selectedPreset.chart_types.map(ct => ct.label).join(', ')}
          </div>
        </div>
      )}
    </div>
  );
}