import { useCallback, useEffect, useMemo, useState } from 'react';
import { useAppStore, useChartSubscription } from '../../store/useAppStore';
import PresetSection from '../PresetSection';
import { Chart } from '@pkg/wasm_bridge.js';
import { parseSymbol } from '../../services/symbolApi';
import SymbolDisplay from './SymbolDisplay';
import SymbolSelector from './SymbolSelector';
import ComparisonModeToggle from './ComparisonModeToggle';
import TimeRangeSelector from './TimeRangeSelector';

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
    resetToDefaults,
    comparisonMode,
    setComparisonMode,
    selectedExchanges,
    setSelectedExchanges,
    toggleExchange,
    baseSymbol: storeBaseSymbol,
    setBaseSymbol
  } = useAppStore();

  // Track subscription events
  const [_subscriptionEvents, setSubscriptionEvents] = useState<ChangeEvent[]>([]);
  const [_activeSubscriptions, setActiveSubscriptions] = useState(0);
  
  // Keep track of the normalized base symbol (e.g., "BTC-USD" not "BTCUSDC")
  const [normalizedBaseSymbol, setNormalizedBaseSymbol] = useState<string>('BTC-USD');
  
  // Parse exchange and base symbol from current symbol
  const { exchange: currentExchange, baseSymbol } = useMemo(() => {
    if (!symbol) return { exchange: 'coinbase', baseSymbol: 'BTC-USD' };
    return parseSymbol(symbol);
  }, [symbol]);
  
  // Get the exact current symbol without the exchange prefix
  const currentSymbolWithoutExchange = baseSymbol;

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
  
  // Initialize selected exchanges when comparison mode is toggled
  useEffect(() => {
    if (comparisonMode && (!selectedExchanges || selectedExchanges.length === 0)) {
      // Find the current exchange-symbol combination in available exchanges
      const currentExchangeSymbol = availableExchanges.find(
        es => es.exchange === currentExchange && es.symbol === currentSymbolWithoutExchange
      );
      
      if (currentExchangeSymbol) {
        // Initialize with the current exchange-symbol combination
        const exchangeSymbolId = `${currentExchangeSymbol.exchange}:${currentExchangeSymbol.symbol}`;
        setSelectedExchanges([exchangeSymbolId]);
      }
    }
  }, [comparisonMode, selectedExchanges, currentExchange, currentSymbolWithoutExchange, availableExchanges, setSelectedExchanges]);
  
  // Only fetch exchanges when we get a new normalized symbol (not exchange-specific)
  useEffect(() => {
    const fetchExchanges = async () => {
      // Determine the normalized symbol to use
      let symbolToCheck = normalizedBaseSymbol;
      
      // If baseSymbol looks like a normalized symbol (contains dash), use it
      if (baseSymbol && baseSymbol.includes('-')) {
        symbolToCheck = baseSymbol;
        setNormalizedBaseSymbol(baseSymbol);
      } else if (storeBaseSymbol && storeBaseSymbol.includes('-')) {
        symbolToCheck = storeBaseSymbol;
        setNormalizedBaseSymbol(storeBaseSymbol);
      }
      
      if (!symbolToCheck) return;
      
      setLoadingExchanges(true);
      try {
        const exchanges = await getAvailableExchanges(symbolToCheck);
        setAvailableExchanges(exchanges);
      } catch (error) {
        console.error('Failed to fetch available exchanges:', error);
        setAvailableExchanges([]);
      } finally {
        setLoadingExchanges(false);
      }
    };
    
    fetchExchanges();
  }, [baseSymbol, storeBaseSymbol, normalizedBaseSymbol]);

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


      {/* Comparison Mode Toggle */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="text-gray-300 text-sm font-medium">Comparison Mode</label>
          <button
            onClick={() => {
              const newComparisonMode = !comparisonMode;
              setComparisonMode(newComparisonMode);
              
              // If turning on comparison mode, initialize with current selection
              if (newComparisonMode && availableExchanges.length > 0) {
                // Find the current exchange-symbol in available exchanges
                const currentMatch = availableExchanges.find(
                  es => es.exchange === currentExchange && es.symbol === currentSymbolWithoutExchange
                );
                
                if (currentMatch) {
                  const exchangeSymbolId = `${currentMatch.exchange}:${currentMatch.symbol}`;
                  setSelectedExchanges([exchangeSymbolId]);
                } else if (availableExchanges.length > 0) {
                  // Fallback to first available exchange-symbol
                  const firstExchange = availableExchanges[0];
                  const exchangeSymbolId = `${firstExchange.exchange}:${firstExchange.symbol}`;
                  setSelectedExchanges([exchangeSymbolId]);
                }
              }
            }}
            className={`
              relative inline-flex h-6 w-11 items-center rounded-full transition-colors
              ${comparisonMode ? 'bg-blue-600' : 'bg-gray-600'}
            `}
          >
            <span
              className={`
                inline-block h-4 w-4 transform rounded-full bg-white transition-transform
                ${comparisonMode ? 'translate-x-6' : 'translate-x-1'}
              `}
            />
          </button>
        </div>
        {comparisonMode && (
          <div className="space-y-1">
            <p className="text-xs text-gray-400">
              Select up to 2 exchanges to compare
            </p>
            <p className="text-xs text-green-500">
              ✓ Multi-exchange comparison enabled
            </p>
          </div>
        )}
      </div>

      {/* Exchange Selection */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">
          {comparisonMode ? 'Select Exchanges' : 'Exchange'}
          {comparisonMode && selectedExchanges && (
            <span className="ml-2 text-xs text-blue-400">
              ({selectedExchanges.length} selected)
            </span>
          )}
        </label>
        {loadingExchanges ? (
          <div className="flex items-center justify-center py-4">
            <div className="text-gray-400 text-sm">Loading exchanges...</div>
          </div>
        ) : availableExchanges.length === 0 ? (
          <div className="text-gray-500 text-sm py-2">
            No exchanges available for this symbol
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-2">
            {availableExchanges.map(exchangeSymbol => {
              const exchange = exchangeSymbol.exchange;
              const exchangeSymbolId = `${exchange}:${exchangeSymbol.symbol}`;
              
              
              // In comparison mode, check if this specific exchange-symbol combo is selected
              const isSelected = comparisonMode && selectedExchanges && selectedExchanges.includes(exchangeSymbolId);
              // Check if this exact exchange-symbol combination is active
              const isActive = !comparisonMode && 
                               currentExchange === exchange && 
                               currentSymbolWithoutExchange === exchangeSymbol.symbol;
              const color = getExchangeColor(exchange);
              
              return (
                <button
                  key={`${exchange}-${exchangeSymbol.symbol}`}
                  onClick={() => {
                    if (comparisonMode) {
                      // In comparison mode, toggle the specific exchange-symbol combination
                      toggleExchange(exchange, exchangeSymbol.symbol);
                    } else {
                      // In single mode, switch to this exchange
                      // Use the specific symbol from this exchange
                      const newSymbol = `${exchange}:${exchangeSymbol.symbol}`;
                      setCurrentSymbol(newSymbol);
                      // Don't update baseSymbol with exchange-specific format
                      // Keep the normalized symbol so the exchange list doesn't change
                      
                      // Update selected exchanges to just this one
                      toggleExchange(exchange, exchangeSymbol.symbol);
                      
                      // Update URL
                      const urlParams = new URLSearchParams(window.location.search);
                      urlParams.set('topic', newSymbol);
                      const newUrl = `${window.location.pathname}?${urlParams.toString()}`;
                      window.history.pushState({}, '', newUrl);
                    }
                  }}
                className={`
                  relative px-3 py-2.5 text-sm font-medium rounded-lg
                  transition-all duration-200 transform
                  ${(isActive || (comparisonMode && isSelected))
                    ? 'bg-gray-700 text-white shadow-lg scale-[1.02]' 
                    : 'bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200'
                  }
                  border ${(isActive || (comparisonMode && isSelected)) ? 'border-gray-500' : 'border-gray-700'}
                  hover:scale-[1.02] active:scale-[0.98]
                `}
                style={{
                  borderLeftWidth: '3px',
                  borderLeftColor: (isActive || (comparisonMode && isSelected)) ? color : 'transparent',
                }}
              >
                <div className="flex items-center justify-between w-full">
                  <div className="flex flex-col items-start">
                    <span className="relative z-10">{formatExchangeName(exchange)}</span>
                    <span className="text-xs text-gray-500 mt-0.5">{exchangeSymbol.symbol}</span>
                  </div>
                  {comparisonMode && (
                    <div className={`
                      w-4 h-4 rounded border-2 flex items-center justify-center flex-shrink-0
                      ${isSelected ? 'bg-blue-600 border-blue-600' : 'border-gray-500'}
                    `}>
                      {isSelected && (
                        <svg className="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 20 20">
                          <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                        </svg>
                      )}
                    </div>
                  )}
                </div>
                {(isActive || (comparisonMode && isSelected)) && (
                  <div 
                    className="absolute inset-0 rounded-lg opacity-10"
                    style={{ backgroundColor: color }}
                  />
                )}
              </button>
            );
            })}
          </div>
        )}
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