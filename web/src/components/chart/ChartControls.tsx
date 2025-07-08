import { useCallback, useEffect, useMemo, useState } from 'react';
import { useAppStore, useChartSubscription } from '../../store/useAppStore';

/**
 * Chart Controls Component
 * 
 * Demonstrates the React store subscription system by providing
 * interactive controls that automatically sync with the WASM chart.
 */
interface ChartControlsProps {
  /** Show detailed subscription information */
  showSubscriptionInfo?: boolean;
  
  /** Enable real-time change tracking */
  enableChangeTracking?: boolean;
}

interface ChangeEvent {
  type: string;
  timestamp: number;
  details: any;
}

export default function ChartControls({ 
  showSubscriptionInfo = false,
  enableChangeTracking = false 
}: ChartControlsProps) {
  const {
    currentSymbol,
    chartConfig,
    isConnected,
    setCurrentSymbol,
    setTimeRange,
    setTimeframe,
    addIndicator,
    removeIndicator,
    addMetric,
    removeMetric,
    updateChartState,
    resetToDefaults
  } = useAppStore();

  // Track subscription events
  const [subscriptionEvents, setSubscriptionEvents] = useState<ChangeEvent[]>([]);
  const [activeSubscriptions, setActiveSubscriptions] = useState(0);

  // Available options (memoized to prevent dependency issues)
  const symbols = useMemo(() => ['BTC-USD', 'ETH-USD', 'ADA-USD', 'DOT-USD', 'LINK-USD', 'AVAX-USD'], []);
  const timeframes = useMemo(() => ['1m', '5m', '15m', '1h', '4h', '1d'], []);
  const availableIndicators = useMemo(() => ['RSI', 'MACD', 'EMA', 'SMA', 'BB', 'STOCH'], []);
  const availableMetrics = useMemo(() => ['best_bid', 'best_ask', 'price', 'volume'], []);

  // Set up chart subscription for change tracking
  const chartSubscription = useChartSubscription({
    onSymbolChange: (newSymbol, oldSymbol) => {
      console.log('[ChartControls] Symbol changed:', { from: oldSymbol, to: newSymbol });
      if (enableChangeTracking) {
        setSubscriptionEvents(prev => [...prev, {
          type: 'Symbol Change',
          timestamp: Date.now(),
          details: { from: oldSymbol, to: newSymbol }
        }]);
      }
    },
    
    onTimeRangeChange: (newRange, oldRange) => {
      console.log('[ChartControls] Time range changed:', { from: oldRange, to: newRange });
      if (enableChangeTracking) {
        setSubscriptionEvents(prev => [...prev, {
          type: 'Time Range Change',
          timestamp: Date.now(),
          details: { from: oldRange, to: newRange }
        }]);
      }
    },
    
    onTimeframeChange: (newTimeframe, oldTimeframe) => {
      console.log('[ChartControls] Timeframe changed:', { from: oldTimeframe, to: newTimeframe });
      if (enableChangeTracking) {
        setSubscriptionEvents(prev => [...prev, {
          type: 'Timeframe Change',
          timestamp: Date.now(),
          details: { from: oldTimeframe, to: newTimeframe }
        }]);
      }
    },
    
    onIndicatorsChange: (newIndicators, oldIndicators) => {
      console.log('[ChartControls] Indicators changed:', { from: oldIndicators, to: newIndicators });
      if (enableChangeTracking) {
        setSubscriptionEvents(prev => [...prev, {
          type: 'Indicators Change',
          timestamp: Date.now(),
          details: { from: oldIndicators, to: newIndicators }
        }]);
      }
    },
    
    onMetricsChange: (newMetrics, oldMetrics) => {
      console.log('[ChartControls] Metrics changed:', { from: oldMetrics, to: newMetrics });
      if (enableChangeTracking) {
        setSubscriptionEvents(prev => [...prev, {
          type: 'Metrics Change',
          timestamp: Date.now(),
          details: { from: oldMetrics, to: newMetrics }
        }]);
      }
    },
    
    onConnectionChange: (connected) => {
      console.log('[ChartControls] Connection changed:', connected);
      if (enableChangeTracking) {
        setSubscriptionEvents(prev => [...prev, {
          type: 'Connection Change',
          timestamp: Date.now(),
          details: { connected }
        }]);
      }
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

  // Indicator management
  const handleIndicatorToggle = useCallback((indicator: string) => {
    if (chartConfig.indicators.includes(indicator)) {
      removeIndicator(indicator);
    } else {
      addIndicator(indicator);
    }
  }, [chartConfig.indicators, addIndicator, removeIndicator]);

  // Metric management
  const handleMetricToggle = useCallback((metric: string) => {
    if (chartConfig.selectedMetrics.includes(metric)) {
      // Don't allow removing all metrics
      if (chartConfig.selectedMetrics.length > 1) {
        removeMetric(metric);
      }
    } else {
      addMetric(metric);
    }
  }, [chartConfig.selectedMetrics, addMetric, removeMetric]);

  // Batch update example
  const handleRandomUpdate = useCallback(() => {
    const randomSymbol = symbols[Math.floor(Math.random() * symbols.length)];
    const randomTimeframe = timeframes[Math.floor(Math.random() * timeframes.length)];
    const randomIndicators = availableIndicators.slice(0, Math.floor(Math.random() * 3) + 1);
    
    updateChartState({
      symbol: randomSymbol,
      timeframe: randomTimeframe,
      indicators: randomIndicators
    });
    
    // Increment update counter in performance metrics
    const currentMetrics = (window as any).__PERFORMANCE_METRICS__ || {};
    const newUpdateCount = (currentMetrics.updateCount || 0) + 1;
    (window as any).__PERFORMANCE_METRICS__ = {
      ...currentMetrics,
      updateCount: newUpdateCount,
      lastStateUpdate: Date.now()
    };
    
    console.log(`[ChartControls] Random update triggered - Update count: ${newUpdateCount}`);
  }, [updateChartState, symbols, timeframes, availableIndicators]);

  // Clear change tracking
  const clearEvents = useCallback(() => {
    setSubscriptionEvents([]);
    setActiveSubscriptions(0);
  }, []);

  return (
    <div className="bg-gray-800 border border-gray-600 rounded-lg p-4 space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-white font-semibold">Chart Controls</h3>
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
          <span className="text-gray-400 text-sm">
            {isConnected ? 'Connected' : 'Disconnected'}
          </span>
        </div>
      </div>
      
      {/* Current Symbol Display */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">Current Symbol</label>
        <div 
          data-testid="current-symbol"
          className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm font-mono"
        >
          {currentSymbol}
        </div>
      </div>

      {/* Symbol Selection */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">Symbol</label>
        <select
          data-testid="symbol-selector"
          value={currentSymbol}
          onChange={(e) => setCurrentSymbol(e.target.value)}
          className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm"
        >
          {symbols.map(symbol => (
            <option key={symbol} value={symbol}>{symbol}</option>
          ))}
        </select>
      </div>
      
      {/* Timeframe Selection */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">Timeframe</label>
        <select
          data-testid="timeframe-selector"
          value={chartConfig.timeframe}
          onChange={(e) => setTimeframe(e.target.value)}
          className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm"
        >
          {timeframes.map(tf => (
            <option key={tf} value={tf}>{tf}</option>
          ))}
        </select>
        <div className="grid grid-cols-3 gap-2">
          {timeframes.map(tf => (
            <button
              key={tf}
              onClick={() => setTimeframe(tf)}
              className={`px-3 py-2 text-sm rounded transition-colors ${
                chartConfig.timeframe === tf
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              }`}
            >
              {tf}
            </button>
          ))}
        </div>
      </div>
      
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
      
      {/* Metrics Selection */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">
          Data Metrics ({chartConfig.selectedMetrics.length})
        </label>
        <div className="grid grid-cols-2 gap-2">
          {availableMetrics.map(metric => (
            <button
              key={metric}
              onClick={() => handleMetricToggle(metric)}
              disabled={chartConfig.selectedMetrics.includes(metric) && chartConfig.selectedMetrics.length === 1}
              className={`px-3 py-2 text-xs rounded transition-colors ${
                chartConfig.selectedMetrics.includes(metric)
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              } ${chartConfig.selectedMetrics.includes(metric) && chartConfig.selectedMetrics.length === 1 ? 'opacity-50 cursor-not-allowed' : ''}`}
              data-testid={`metric-${metric}`}
            >
              {metric.replace('_', ' ').toUpperCase()}
            </button>
          ))}
        </div>
        <div className="text-xs text-gray-500">
          Select multiple metrics to overlay on the chart
        </div>
      </div>

      {/* Indicators */}
      <div className="space-y-2">
        <label className="text-gray-300 text-sm font-medium">
          Indicators ({chartConfig.indicators.length})
        </label>
        <div className="grid grid-cols-3 gap-2">
          {availableIndicators.map(indicator => (
            <button
              key={indicator}
              onClick={() => handleIndicatorToggle(indicator)}
              className={`px-2 py-1 text-xs rounded transition-colors ${
                chartConfig.indicators.includes(indicator)
                  ? 'bg-green-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              }`}
            >
              {indicator}
            </button>
          ))}
        </div>
      </div>
      
      {/* Action Buttons */}
      <div className="space-y-2">
        <div className="grid grid-cols-2 gap-2">
          <button
            onClick={handleRandomUpdate}
            className="px-4 py-2 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 transition-colors"
          >
            Random Update
          </button>
          
          <button
            data-testid="reset-button"
            onClick={resetToDefaults}
            className="px-4 py-2 bg-gray-600 text-white text-sm rounded hover:bg-gray-700 transition-colors"
          >
            Reset Defaults
          </button>
        </div>
      </div>
      
      {/* Subscription Information */}
      {showSubscriptionInfo && (
        <div className="border-t border-gray-600 pt-4">
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-gray-300 text-sm font-medium">Subscription Info</h4>
            <span className="text-gray-400 text-xs">
              {activeSubscriptions} changes
            </span>
          </div>
          
          <div className="space-y-2 text-xs">
            <div className="flex justify-between">
              <span className="text-gray-400">Auto Sync:</span>
              <span className="text-green-500">Enabled</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Debounce:</span>
              <span className="text-gray-300">100ms</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-400">Last Update:</span>
              <span className="text-gray-300">
                {subscriptionEvents.length > 0 
                  ? new Date(subscriptionEvents[subscriptionEvents.length - 1].timestamp).toLocaleTimeString()
                  : 'Never'
                }
              </span>
            </div>
          </div>
        </div>
      )}
      
      {/* Change Tracking */}
      {enableChangeTracking && (
        <div className="border-t border-gray-600 pt-4">
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-gray-300 text-sm font-medium">
              Change Events ({subscriptionEvents.length})
            </h4>
            <button
              onClick={clearEvents}
              className="text-gray-400 hover:text-white text-xs"
            >
              Clear
            </button>
          </div>
          
          <div className="max-h-40 overflow-y-auto space-y-1">
            {subscriptionEvents.slice(-10).reverse().map((event, index) => (
              <div key={index} className="bg-gray-700 rounded p-2 text-xs">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-blue-400 font-medium">{event.type}</span>
                  <span className="text-gray-500">
                    {new Date(event.timestamp).toLocaleTimeString()}
                  </span>
                </div>
                <div className="text-gray-300">
                  {JSON.stringify(event.details, null, 0)}
                </div>
              </div>
            ))}
            
            {subscriptionEvents.length === 0 && (
              <div className="text-gray-500 text-center py-4">
                No changes tracked yet
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}