import { useCallback, useEffect, useMemo, useState } from 'react';
import { useAppStore, useChartSubscription } from '../../store/useAppStore';
import { useLoading } from '../../contexts/LoadingContext';
import PresetSection from '../PresetSection';
import { Chart } from '@pkg/wasm_bridge.js';
import { parseSymbol } from '../../services/symbolApi';
import { UIErrorBoundary } from '../error/UIErrorBoundary';
import { ControlsLoadingSkeleton } from '../loading/LoadingSkeleton';
import SymbolDisplay from './SymbolDisplay';
import SymbolSelector from './SymbolSelector';
import ComparisonModeToggle from './ComparisonModeToggle';
import TimeRangeSelector from './TimeRangeSelector';

/**
 * Refactored Chart Controls Component
 * 
 * Split into smaller, focused components for better maintainability:
 * - SymbolDisplay: Shows current symbol with exchange info
 * - SymbolSelector: Dropdown for symbol selection
 * - ComparisonModeToggle: Handles comparison mode and exchange selection
 * - TimeRangeSelector: Time range preset buttons
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

function ChartControlsCore({
  chartInstance,
  appliedPreset,
  onPresetChange
}: ChartControlsProps) {
  const { loading } = useLoading();
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
    baseSymbol: _storeBaseSymbol,
    setBaseSymbol: _setBaseSymbol,
    startTime: _startTime
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
    onSymbolChange: (_newSymbol, _oldSymbol) => {
      // Handle symbol changes if needed
    },

    onTimeRangeChange: (_newRange, _oldRange) => {
      // Handle time range changes if needed
    },

    onPresetChange(_newPreset, _oldPreset) {
      // Handle preset changes if needed
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

  // Update normalized base symbol when baseSymbol changes
  useEffect(() => {
    if (baseSymbol && baseSymbol !== normalizedBaseSymbol) {
      setNormalizedBaseSymbol(baseSymbol);
    }
  }, [baseSymbol, normalizedBaseSymbol]);

  // Time range handler for TimeRangeSelector component
  const handleTimeRangeChange = useCallback((startTime: number, endTime: number) => {
    setTimeRange(startTime, endTime);
  }, [setTimeRange]);

  // Show loading skeleton while initializing
  if (loading.wasm || loading.initialization || loading.webgpu) {
    return <ControlsLoadingSkeleton />;
  }

  return (
    <div 
      className="bg-gray-800 border border-gray-600 rounded-lg p-4 space-y-6"
      role="group"
      aria-labelledby="chart-controls-heading"
    >
      <div className="flex items-center justify-between">
        <h3 id="chart-controls-heading" className="text-white font-semibold">Chart Controls</h3>
        <button
          onClick={resetToDefaults}
          className="text-xs text-gray-400 hover:text-gray-200 px-2 py-1 rounded hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
          aria-label="Reset chart to default settings"
        >
          Reset
        </button>
      </div>

      {/* Current Symbol Display */}
      <SymbolDisplay 
        baseSymbol={baseSymbol || ''}
        exchange={currentExchange}
      />

      {/* Symbol Selection */}
      <SymbolSelector 
        symbol={symbol}
        onSymbolChange={setCurrentSymbol}
      />

      {/* Preset Selection */}
      {chartInstance && (
        <PresetSection
          chartInstance={chartInstance}
          onPresetChange={onPresetChange}
          preset={appliedPreset}
        />
      )}

      {/* Comparison Mode Toggle */}
      <ComparisonModeToggle
        comparisonMode={comparisonMode}
        onComparisonModeChange={setComparisonMode}
        selectedExchanges={selectedExchanges}
        onSelectedExchangesChange={setSelectedExchanges}
        onToggleExchange={toggleExchange}
        onSymbolChange={setCurrentSymbol}
        currentExchange={currentExchange}
        currentSymbolWithoutExchange={currentSymbolWithoutExchange}
        normalizedBaseSymbol={normalizedBaseSymbol}
      />

      {/* Time Range Presets */}
      <TimeRangeSelector 
        onTimeRangeChange={handleTimeRangeChange}
        currentStartTime={_startTime}
      />
    </div>
  );
}

export default function ChartControls(props: ChartControlsProps) {
  return (
    <UIErrorBoundary componentName="ChartControls">
      <ChartControlsCore {...props} />
    </UIErrorBoundary>
  );
}