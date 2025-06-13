import { create } from 'zustand';
import { AppState, ChartConfig, MarketData } from '../types';

interface StoreSubscriptionCallbacks {
  onSymbolChange?: (newSymbol: string, oldSymbol: string) => void;
  onTimeRangeChange?: (newRange: { startTime: number; endTime: number }, oldRange: { startTime: number; endTime: number }) => void;
  onTimeframeChange?: (newTimeframe: string, oldTimeframe: string) => void;
  onIndicatorsChange?: (newIndicators: string[], oldIndicators: string[]) => void;
  onConnectionChange?: (connected: boolean) => void;
  onMarketDataChange?: (symbol: string, data: MarketData) => void;
  onAnyChange?: (newState: AppState, oldState: AppState) => void;
}

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
      if (!oldState || newState._subscriptions.size === 0) return;
      
      // Detect specific changes
      const symbolChanged = newState.currentSymbol !== oldState.currentSymbol;
      const timeRangeChanged = newState.chartConfig.startTime !== oldState.chartConfig.startTime ||
                              newState.chartConfig.endTime !== oldState.chartConfig.endTime;
      const timeframeChanged = newState.chartConfig.timeframe !== oldState.chartConfig.timeframe;
      const indicatorsChanged = JSON.stringify(newState.chartConfig.indicators) !== 
                               JSON.stringify(oldState.chartConfig.indicators);
      
      // Trigger specific callbacks
      newState._subscriptions.forEach((callbacks) => {
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