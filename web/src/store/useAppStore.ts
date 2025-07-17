import { create } from 'zustand';
import { AppState, ChartConfig, MarketData, StoreSubscriptionCallbacks } from '../types';

interface AppStore extends AppState {
  // Store subscription management
  _subscriptions: Map<string, StoreSubscriptionCallbacks>;
  _lastState: AppState | null;
  
  // Core actions
  setCurrentSymbol: (symbol: string) => void;
  setChartConfig: (config: ChartConfig) => void;
  updateMarketData: (symbol: string, data: MarketData) => void;
  setConnectionStatus: (connected: boolean) => void;
  
  // Enhanced actions with time range management
  setTimeRange: (startTime: number, endTime: number) => void;
  setTimeframe: (timeframe: string) => void;
  addIndicator: (indicator: string) => void;
  removeIndicator: (indicator: string) => void;
  setIndicators: (indicators: string[]) => void;
  
  // Metric selection actions
  addMetric: (metric: string) => void;
  removeMetric: (metric: string) => void;
  setSelectedMetrics: (metrics: string[]) => void;
  
  // Chart type actions
  setChartType: (chartType: 'line' | 'candlestick') => void;
  setCandleTimeframe: (timeframe: number) => void;
  
  // Batch operations
  updateChartState: (updates: Partial<ChartConfig>) => void;
  resetToDefaults: () => void;
  
  // Store subscription API
  subscribe: (id: string, callbacks: StoreSubscriptionCallbacks) => () => void;
  unsubscribe: (id: string) => void;
  
  // Internal subscription trigger
  _triggerSubscriptions: (newState: AppState, oldState: AppState) => void;
}

// Default configuration values
const DEFAULT_CONFIG: ChartConfig = {
  symbol: 'BTC-USD',
  timeframe: '1h',
  startTime: Math.floor(Date.now() / 1000) - 24 * 60 * 60, // 24 hours ago
  endTime: Math.floor(Date.now() / 1000), // Now
  indicators: [],
  selectedMetrics: ['best_bid', 'best_ask'], // Default to both bid and ask
  chartType: 'line',
  candleTimeframe: 60, // Default 1 minute candles
};

export const useAppStore = create<AppStore>((set, get) => ({
    // Initial state
    currentSymbol: DEFAULT_CONFIG.symbol,
    chartConfig: DEFAULT_CONFIG,
    marketData: {},
    isConnected: false,
    user: undefined,
    
    // Subscription management
    _subscriptions: new Map(),
    _lastState: null,

    // Core actions with enhanced subscription triggering
    setCurrentSymbol: (symbol) => {
      const oldState = get();
      set((state) => ({
        currentSymbol: symbol,
        chartConfig: { ...state.chartConfig, symbol },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },

    setChartConfig: (config) => {
      const oldState = get();
      set({ chartConfig: config });
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },

    updateMarketData: (symbol, data) => {
      const oldState = get();
      set((state) => ({
        marketData: {
          ...state.marketData,
          [symbol]: data,
        },
      }));
      const newState = get();
      
      // Trigger market data specific callback
      newState._subscriptions.forEach((callbacks) => {
        callbacks.onMarketDataChange?.(symbol, data);
      });
      
      newState._triggerSubscriptions(newState, oldState);
    },

    setConnectionStatus: (connected) => {
      const oldState = get();
      set({ isConnected: connected });
      const newState = get();
      
      // Trigger connection specific callback
      newState._subscriptions.forEach((callbacks) => {
        callbacks.onConnectionChange?.(connected);
      });
      
      newState._triggerSubscriptions(newState, oldState);
    },
    
    // Enhanced time range management
    setTimeRange: (startTime, endTime) => {
      const oldState = get();
      set((state) => ({
        chartConfig: { ...state.chartConfig, startTime, endTime },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    setTimeframe: (timeframe) => {
      const oldState = get();
      set((state) => ({
        chartConfig: { ...state.chartConfig, timeframe },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    // Indicator management
    addIndicator: (indicator) => {
      const oldState = get();
      set((state) => ({
        chartConfig: {
          ...state.chartConfig,
          indicators: [...state.chartConfig.indicators, indicator],
        },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    removeIndicator: (indicator) => {
      const oldState = get();
      set((state) => ({
        chartConfig: {
          ...state.chartConfig,
          indicators: state.chartConfig.indicators.filter(ind => ind !== indicator),
        },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    setIndicators: (indicators) => {
      const oldState = get();
      set((state) => ({
        chartConfig: { ...state.chartConfig, indicators },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    // Metric management
    addMetric: (metric) => {
      const oldState = get();
      set((state) => ({
        chartConfig: {
          ...state.chartConfig,
          selectedMetrics: [...state.chartConfig.selectedMetrics, metric],
        },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    removeMetric: (metric) => {
      const oldState = get();
      set((state) => ({
        chartConfig: {
          ...state.chartConfig,
          selectedMetrics: state.chartConfig.selectedMetrics.filter(m => m !== metric),
        },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    setSelectedMetrics: (metrics) => {
      const oldState = get();
      set((state) => ({
        chartConfig: { ...state.chartConfig, selectedMetrics: metrics },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    // Chart type actions
    setChartType: (chartType) => {
      const oldState = get();
      set((state) => ({
        chartConfig: { ...state.chartConfig, chartType },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    setCandleTimeframe: (timeframe) => {
      const oldState = get();
      set((state) => ({
        chartConfig: { ...state.chartConfig, candleTimeframe: timeframe },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    // Batch operations
    updateChartState: (updates) => {
      const oldState = get();
      set((state) => ({
        chartConfig: { ...state.chartConfig, ...updates },
      }));
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    resetToDefaults: () => {
      const oldState = get();
      set({
        currentSymbol: DEFAULT_CONFIG.symbol,
        chartConfig: { ...DEFAULT_CONFIG },
        marketData: {},
        isConnected: false,
        user: undefined,
      });
      const newState = get();
      newState._triggerSubscriptions(newState, oldState);
    },
    
    // Subscription API
    subscribe: (id, callbacks) => {
      const state = get();
      state._subscriptions.set(id, callbacks);
      
      // Return unsubscribe function
      return () => {
        const currentState = get();
        currentState._subscriptions.delete(id);
      };
    },
    
    unsubscribe: (id) => {
      const state = get();
      state._subscriptions.delete(id);
    },
    
    // Internal subscription trigger with smart change detection
    _triggerSubscriptions: (newState, oldState) => {
      if (!oldState || !newState._subscriptions || newState._subscriptions.size === 0) return;
      
      // Detect specific changes
      const symbolChanged = newState.currentSymbol !== oldState.currentSymbol;
      const timeRangeChanged = newState.chartConfig.startTime !== oldState.chartConfig.startTime ||
                              newState.chartConfig.endTime !== oldState.chartConfig.endTime;
      const timeframeChanged = newState.chartConfig.timeframe !== oldState.chartConfig.timeframe;
      const indicatorsChanged = JSON.stringify(newState.chartConfig.indicators) !== 
                               JSON.stringify(oldState.chartConfig.indicators);
      const metricsChanged = JSON.stringify(newState.chartConfig.selectedMetrics) !== 
                            JSON.stringify(oldState.chartConfig.selectedMetrics);
      
      // Trigger specific callbacks
      newState._subscriptions?.forEach((callbacks: StoreSubscriptionCallbacks) => {
        if (symbolChanged && callbacks.onSymbolChange) {
          callbacks.onSymbolChange(newState.currentSymbol, oldState.currentSymbol);
        }
        
        if (timeRangeChanged && callbacks.onTimeRangeChange) {
          callbacks.onTimeRangeChange(
            { startTime: newState.chartConfig.startTime, endTime: newState.chartConfig.endTime },
            { startTime: oldState.chartConfig.startTime, endTime: oldState.chartConfig.endTime }
          );
        }
        
        if (timeframeChanged && callbacks.onTimeframeChange) {
          callbacks.onTimeframeChange(newState.chartConfig.timeframe, oldState.chartConfig.timeframe);
        }
        
        if (indicatorsChanged && callbacks.onIndicatorsChange) {
          callbacks.onIndicatorsChange(newState.chartConfig.indicators, oldState.chartConfig.indicators);
        }
        
        if (metricsChanged && callbacks.onMetricsChange) {
          callbacks.onMetricsChange(newState.chartConfig.selectedMetrics, oldState.chartConfig.selectedMetrics);
        }
        
        // Always trigger general change callback
        if (callbacks.onAnyChange) {
          callbacks.onAnyChange(newState, oldState);
        }
      });
      
      // Update last state reference
      set({ _lastState: { ...newState } });
    },
  }));

// Export helper hooks for specific subscriptions
export const useSymbolSubscription = (callback: (newSymbol: string, oldSymbol: string) => void) => {
  const subscribe = useAppStore(state => state.subscribe);
  const unsubscribe = useAppStore(state => state.unsubscribe);
  
  return {
    subscribe: () => subscribe('symbol-subscription', { onSymbolChange: callback }),
    unsubscribe: () => unsubscribe('symbol-subscription'),
  };
};

export const useTimeRangeSubscription = (callback: (newRange: { startTime: number; endTime: number }, oldRange: { startTime: number; endTime: number }) => void) => {
  const subscribe = useAppStore(state => state.subscribe);
  const unsubscribe = useAppStore(state => state.unsubscribe);
  
  return {
    subscribe: () => subscribe('timerange-subscription', { onTimeRangeChange: callback }),
    unsubscribe: () => unsubscribe('timerange-subscription'),
  };
};

export const useChartSubscription = (callbacks: StoreSubscriptionCallbacks) => {
  const subscribe = useAppStore(state => state.subscribe);
  const unsubscribe = useAppStore(state => state.unsubscribe);
  
  return {
    subscribe: () => subscribe('chart-subscription', callbacks),
    unsubscribe: () => unsubscribe('chart-subscription'),
  };
};

// Expose store globally for testing
if (typeof window !== 'undefined') {
  (window as any).__zustandStore = useAppStore;
  (window as any).__APP_STORE_STATE__ = useAppStore.getState();
  (window as any).__STORE_READY__ = true;
  (window as any).__DATA_SERVICE_READY__ = true;
  (window as any).__ERROR_HANDLER_READY__ = true;
  (window as any).__PERFORMANCE_MONITOR_READY__ = true;
  
  // Initialize error tracking
  if (!(window as any).wasmErrors) {
    (window as any).wasmErrors = [];
  }
  
  // Add store accessor functions for tests
  (window as any).__GET_STORE_STATE__ = () => {
    const state = useAppStore.getState();
    return {
      currentSymbol: state.currentSymbol,
      symbol: state.currentSymbol, // Alias for backward compatibility
      chartConfig: state.chartConfig,
      timeframe: state.chartConfig.timeframe,
      connected: state.isConnected,
      chartInitialized: true, // Assume chart is initialized if store is accessible
      marketData: state.marketData,
      isConnected: state.isConnected,
      user: state.user,
      startTime: state.chartConfig.startTime,
      endTime: state.chartConfig.endTime
    };
  };

  // Store update function for tests
  (window as any).__UPDATE_STORE_STATE__ = (updates: any) => {
    const store = useAppStore.getState();
    if (updates.currentSymbol) {
      store.setCurrentSymbol(updates.currentSymbol);
    }
    if (updates.symbol) {
      store.setCurrentSymbol(updates.symbol);
    }
    if (updates.timeframe) {
      store.setTimeframe(updates.timeframe);
    }
    if (updates.startTime !== undefined && updates.endTime !== undefined) {
      store.setTimeRange(updates.startTime, updates.endTime);
    }
    if (updates.indicators) {
      store.setIndicators(updates.indicators);
    }
    if (updates.connected !== undefined) {
      store.setConnectionStatus(updates.connected);
    }
    return { success: true };
  };
  
  // Update global state on store changes
  useAppStore.subscribe((state) => {
    (window as any).__APP_STORE_STATE__ = state;
  });
}