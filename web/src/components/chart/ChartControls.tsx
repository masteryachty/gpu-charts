import { useCallback, useEffect, useMemo, useState } from 'react';
import { useAppStore, useChartSubscription } from '../../store/useAppStore';
import PresetSection from '../PresetSection';
import { Chart } from '@pkg/wasm_bridge.js';
import { formatExchangeName, getExchangeColor, parseSymbol } from '../../services/symbolApi';

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

  /** Applied preset name (after it's been set in WASM) */
  appliedPreset?: string;

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
  appliedPreset,
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
  
  // Parse exchange and base symbol from current symbol
  const { exchange: currentExchange, baseSymbol } = useMemo(() => {
    if (!symbol) return { exchange: 'coinbase', baseSymbol: 'BTC-USD' };
    return parseSymbol(symbol);
  }, [symbol]);
  
  // Available exchanges
  const exchanges = useMemo(() => [
    { id: 'coinbase', name: 'Coinbase' },
    { id: 'binance', name: 'Binance' },
    { id: 'kraken', name: 'Kraken' },
    { id: 'bitfinex', name: 'Bitfinex' },
    { id: 'okx', name: 'OKX' },
  ], []);

  // Set up chart subscription for change tracking
  const chartSubscription = useChartSubscription({
    onSymbolChange: (newSymbol, oldSymbol) => {

    },

    onTimeRangeChange: (newRange, oldRange) => {
    },

    onPresetChange(newPreset, oldPreset) {
    },

    onAnyChange: (_newState, _oldState) => {
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
          className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm"
        >
          <div className="flex items-center justify-between">
            <span className="font-mono">{baseSymbol}</span>
            <span 
              className="text-xs px-2 py-1 rounded"
              style={{
                backgroundColor: `${getExchangeColor(currentExchange)}20`,
                color: getExchangeColor(currentExchange),
                border: `1px solid ${getExchangeColor(currentExchange)}40`
              }}
            >
              {formatExchangeName(currentExchange)}
            </span>
          </div>
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
          preset={appliedPreset}
        />
      )}


      {/* Exchange Selection */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">Exchange</label>
        <div className="grid grid-cols-2 gap-2">
          {exchanges.map(exchange => {
            const isActive = currentExchange === exchange.id;
            const color = getExchangeColor(exchange.id);
            return (
              <button
                key={exchange.id}
                onClick={() => {
                  const newSymbol = `${exchange.id}:${baseSymbol || 'BTC-USD'}`;
                  setCurrentSymbol(newSymbol);
                  
                  // Update URL
                  const urlParams = new URLSearchParams(window.location.search);
                  urlParams.set('topic', newSymbol);
                  const newUrl = `${window.location.pathname}?${urlParams.toString()}`;
                  window.history.pushState({}, '', newUrl);
                }}
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
                style={{
                  borderLeftWidth: '3px',
                  borderLeftColor: isActive ? color : 'transparent',
                }}
              >
                <span className="relative z-10">{exchange.name}</span>
                {isActive && (
                  <div 
                    className="absolute inset-0 rounded-lg opacity-10"
                    style={{ backgroundColor: color }}
                  />
                )}
              </button>
            );
          })}
        </div>
      </div>

      {/* Time Range Presets */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">Time Range</label>
        <div className="grid grid-cols-2 gap-2">
          {[
            { id: '1h', label: '1 Hour' },
            { id: '4h', label: '4 Hours' },
            { id: '1d', label: '1 Day' },
            { id: '1w', label: '1 Week' }
          ].map(preset => {
            // Check if this preset is currently active
            const now = Math.floor(Date.now() / 1000);
            let presetStart: number;
            switch (preset.id) {
              case '1h': presetStart = now - 3600; break;
              case '4h': presetStart = now - 14400; break;
              case '1d': presetStart = now - 86400; break;
              case '1w': presetStart = now - 604800; break;
              default: presetStart = now - 86400;
            }
            // Simple check if current range matches (within 60 seconds tolerance)
            const isActive = Math.abs(useAppStore.getState().startTime - presetStart) < 60;
            
            return (
              <button
                key={preset.id}
                onClick={() => handleTimeRangePreset(preset.id)}
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
                {preset.label}
              </button>
            );
          })}
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