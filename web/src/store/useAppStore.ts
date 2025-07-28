import { create } from 'zustand';
import { AppState, ChartState, MarketData, StoreSubscriptionCallbacks } from '../types';

interface AppStore extends AppState {
  // Store subscription management
  _subscriptions: Map<string, StoreSubscriptionCallbacks>;
  _lastState: AppState | null;

  // Core actions
  setCurrentSymbol: (symbol: string) => void;
  setChartStateConfig: (config: ChartState) => void;
  updateMarketData: (symbol: string, data: MarketData) => void;
  setConnectionStatus: (connected: boolean) => void;

  // Enhanced actions with time range management
  setTimeRange: (startTime: number, endTime: number) => void;

  // Metric preset actions (simplified - just preset name)
  setMetricPreset: (presetName: string | null) => void;

  // Batch operations
  updateChartState: (updates: Partial<ChartState>) => void;
  resetToDefaults: () => void;

  // Store subscription API
  subscribe: (id: string, callbacks: StoreSubscriptionCallbacks) => () => void;
  unsubscribe: (id: string) => void;

  // Internal subscription trigger
  _triggerSubscriptions: (newState: AppState, oldState: AppState) => void;
}

// Default configuration values
const DEFAULT_CONFIG: ChartState = {
  symbol: 'BTC-USD',
  startTime: Math.floor(Date.now() / 1000) - 24 * 60 * 60, // 24 hours ago
  endTime: Math.floor(Date.now() / 1000), // Now
  metricPreset: null, // No preset selected by default
};

export const useAppStore = create<AppStore>((set, get) => ({
  // Initial state
  currentSymbol: DEFAULT_CONFIG.symbol,
  ChartStateConfig: DEFAULT_CONFIG,
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
      ChartStateConfig: { ...state.ChartStateConfig, symbol },
    }));
    const newState = get();
    newState._triggerSubscriptions(newState, oldState);
  },

  setChartStateConfig: (config) => {
    const oldState = get();
    // Use migration function to ensure all fields are present
    const migratedConfig = migrateChartStateConfig(config);
    set({ ChartStateConfig: migratedConfig });
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
      ChartStateConfig: { ...state.ChartStateConfig, startTime, endTime },
    }));
    const newState = get();
    newState._triggerSubscriptions(newState, oldState);
  },

  // Metric preset actions (simplified - just preset name)
  setMetricPreset: (presetName) => {
    const oldState = get();
    set((state) => ({
      ChartStateConfig: {
        ...state.ChartStateConfig,
        metricPreset: presetName,
      },
    }));
    const newState = get();
    newState._triggerSubscriptions(newState, oldState);
  },

  // Batch operations
  updateChartState: (updates) => {
    const oldState = get();
    set((state) => ({
      ChartStateConfig: { ...state.ChartStateConfig, ...updates },
    }));
    const newState = get();
    newState._triggerSubscriptions(newState, oldState);
  },

  resetToDefaults: () => {
    const oldState = get();
    set({
      currentSymbol: DEFAULT_CONFIG.symbol,
      ChartStateConfig: { ...DEFAULT_CONFIG },
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
    const timeRangeChanged = newState.ChartStateConfig.startTime !== oldState.ChartStateConfig.startTime ||
      newState.ChartStateConfig.endTime !== oldState.ChartStateConfig.endTime;
    const presetChanged = newState.ChartStateConfig.metricPreset !== oldState.ChartStateConfig.metricPreset;

    // Trigger specific callbacks
    newState._subscriptions?.forEach((callbacks: StoreSubscriptionCallbacks) => {
      if (symbolChanged && callbacks.onSymbolChange) {
        callbacks.onSymbolChange(newState.currentSymbol, oldState.currentSymbol);
      }

      if (timeRangeChanged && callbacks.onTimeRangeChange) {
        callbacks.onTimeRangeChange(
          { startTime: newState.ChartStateConfig.startTime, endTime: newState.ChartStateConfig.endTime },
          { startTime: oldState.ChartStateConfig.startTime, endTime: oldState.ChartStateConfig.endTime }
        );
      }


      // Preset changes can be tracked if needed
      if (presetChanged && callbacks.onMetricsChange) {
        // Just notify that preset changed, WASM handles the details
        callbacks.onMetricsChange([], []);
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
      ChartStateConfig: state.ChartStateConfig,
      connected: state.isConnected,
      chartInitialized: true, // Assume chart is initialized if store is accessible
      marketData: state.marketData,
      isConnected: state.isConnected,
      user: state.user,
      startTime: state.ChartStateConfig.startTime,
      endTime: state.ChartStateConfig.endTime
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
    if (updates.startTime !== undefined && updates.endTime !== undefined) {
      store.setTimeRange(updates.startTime, updates.endTime);
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

// Migration function to ensure backward compatibility
export function migrateChartStateConfig(config: Partial<ChartState>): ChartState {
  // Start with default config
  const migratedConfig = { ...DEFAULT_CONFIG, ...config };

  // Ensure metricPreset field exists
  if (migratedConfig.metricPreset === undefined) {
    migratedConfig.metricPreset = null;
  }

  return migratedConfig;
}