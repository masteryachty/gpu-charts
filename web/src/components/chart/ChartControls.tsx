import { useCallback, useEffect, useMemo, useState } from 'react';
import { useAppStore, useChartSubscription } from '../../store/useAppStore';
import PresetSection from '../PresetSection';
import { Chart } from '@pkg/wasm_bridge.js';

/**
 * Chart Controls Component
 * 
 * Simplified controls that work with the new WASM architecture:
 * - React state only contains: metricPreset, symbol, startTime, endTime
 * - Preset selection calls WASM apply_preset() which fetches data automatically
 * - Metric visibility toggles call WASM toggle_metric_visibility()
 */
interface ChartControlsProps {
  /** WASM Chart instance */
  chartInstance?: Chart;

  /** Show detailed subscription information */
  showSubscriptionInfo?: boolean;

  /** Enable real-time change tracking */
  enableChangeTracking?: boolean;

  /** Callback when preset changes */
  onPresetChange: (presetName?: string) => void;
}

interface ChangeEvent {
  type: string;
  timestamp: number;
  details: any;
}

export default function ChartControls({
  chartInstance,
  onPresetChange
}: ChartControlsProps) {
  const {
    symbol,
    preset,
    setCurrentSymbol,
    setTimeRange,
    resetToDefaults
  } = useAppStore();

  // Track subscription events
  const [_subscriptionEvents, setSubscriptionEvents] = useState<ChangeEvent[]>([]);
  const [_activeSubscriptions, setActiveSubscriptions] = useState(0);

  // Available options (memoized to prevent dependency issues)
  const symbols = useMemo(() => ['BTC-USD', 'ETH-USD', 'ADA-USD', 'DOT-USD', 'LINK-USD', 'AVAX-USD'], []);

  // Set up chart subscription for change tracking
  const chartSubscription = useChartSubscription({
    onSymbolChange: (newSymbol, oldSymbol) => {
      console.log('[ChartControls] Symbol changed:', { from: oldSymbol, to: newSymbol });

    },

    onTimeRangeChange: (newRange, oldRange) => {
      console.log('[ChartControls] Time range changed:', { from: oldRange, to: newRange });
    },

    onPresetChange(newPreset, oldPreset) {
      console.log('[ChartControls] Preset changed:', { from: oldPreset, to: newPreset });
    },

    onAnyChange: (_newState, _oldState) => {
      console.log('[ChartControls] Store state changed');
      setActiveSubscriptions(prev => prev + 1);
    }
  });

  // Subscribe on mount
  useEffect(() => {
    const unsubscribe = chartSubscription.subscribe();
    return unsubscribe;
  }, [chartSubscription]);

  // Time range controls
  const handleTimeRangePreset = useCallback((preset: string) => {
    const now = Math.floor(Date.now() / 1000);
    let startTime: number;

    switch (preset) {
      case '1h':
        startTime = now - 3600;
        break;
      case '4h':
        startTime = now - 14400;
        break;
      case '1d':
        startTime = now - 86400;
        break;
      case '1w':
        startTime = now - 604800;
        break;
      default:
        startTime = now - 86400;
    }

    setTimeRange(startTime, now);
  }, [setTimeRange]);


  return (
    <div className="bg-gray-800 border border-gray-600 rounded-lg p-4 space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-white font-semibold">Chart Controls</h3>
      </div>

      {/* Current Symbol Display */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">Current Symbol</label>
        <div
          data-testid="current-symbol"
          className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm font-mono"
        >
          {symbol}
        </div>
      </div>

      {/* Symbol Selection */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">Symbol</label>
        <select
          data-testid="symbol-selector"
          value={symbol}
          onChange={(e) => setCurrentSymbol(e.target.value)}
          className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm"
        >
          {symbols.map(symbol => (
            <option key={symbol} value={symbol}>{symbol}</option>
          ))}
        </select>
      </div>

      {/* Preset Selection */}
      {chartInstance && (
        <PresetSection
          chartInstance={chartInstance}
          preset={preset}
        />
      )}


      {/* Time Range Presets */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">Time Range</label>
        <div className="grid grid-cols-2 gap-2">
          {['1h', '4h', '1d', '1w'].map(preset => (
            <button
              key={preset}
              onClick={() => handleTimeRangePreset(preset)}
              className="px-3 py-2 text-sm bg-gray-700 text-gray-300 rounded hover:bg-gray-600 transition-colors"
            >
              Last {preset}
            </button>
          ))}
        </div>
      </div>



      {/* Action Buttons */}
      <div className="space-y-2">
        <button
          data-testid="reset-button"
          onClick={resetToDefaults}
          className="w-full px-4 py-2 bg-gray-600 text-white text-sm rounded hover:bg-gray-700 transition-colors"
        >
          Reset Defaults
        </button>
      </div>

    </div>
  );
}