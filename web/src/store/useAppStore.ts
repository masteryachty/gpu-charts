import { create } from 'zustand';

// Simplified StoreState matching Rust expectations
export interface StoreState {
  preset?: string;
  symbol?: string;
  startTime: number;
  endTime: number;
}

// // WASM integration types
// export interface WasmModule {
//   memory: WebAssembly.Memory;
//   // Add other WASM exports as needed
// }

// Store subscription callback interface
export interface StoreSubscriptionCallbacks {
  onSymbolChange?: (newSymbol?: string, oldSymbol?: string) => void;
  onTimeRangeChange?: (newRange: { startTime: number; endTime: number }, oldRange: { startTime: number; endTime: number }) => void;
  onPresetChange?: (newPreset?: string, oldPreset?: string) => void;
  onAnyChange?: (newState: StoreState, oldState: StoreState) => void;
}

interface AppStore extends StoreState {
  // Store subscription management
  _subscriptions: Map<string, StoreSubscriptionCallbacks>;
  _lastState: StoreState | null;

  // Core actions
  setCurrentSymbol: (symbol: string) => void;
  setPreset: (preset?: string) => void;
  // Enhanced actions with time range management
  setTimeRange: (startTime: number, endTime: number) => void;
  // Batch operations
  updateChartState: (updates: Partial<StoreState>) => void;
  resetToDefaults: () => void;

  // Store subscription API
  subscribe: (id: string, callbacks: StoreSubscriptionCallbacks) => () => void;
  unsubscribe: (id: string) => void;

  // Internal subscription trigger
  _triggerSubscriptions: (newState: AppStore, oldState: AppStore) => void;
}

// Default configuration values
const DEFAULT_CONFIG: StoreState = {
  symbol: 'BTC-USD',
  startTime: Math.floor(Date.now() / 1000) - 24 * 60 * 60, // 24 hours ago
  endTime: Math.floor(Date.now() / 1000), // Now
  preset: 'Market Data',
};

export const useAppStore = create<AppStore>((set, get) => ({

  

  // Initial state
  symbol: DEFAULT_CONFIG.symbol,
  preset: DEFAULT_CONFIG.preset,
  startTime: DEFAULT_CONFIG.startTime,
  endTime: DEFAULT_CONFIG.endTime,

  // Subscription management
  _subscriptions: new Map(),
  _lastState: null,

  // Core actions with enhanced subscription triggering
  setCurrentSymbol: (symbol) => {
    const oldState = get();
    set({ symbol });
    const newState = get();
    newState._triggerSubscriptions(newState, oldState);
  },

  setPreset: (preset) => {
    const oldState = get();
    set({ preset });
    const newState = get();
    newState._triggerSubscriptions(newState, oldState);
  },

  // Enhanced time range management
  setTimeRange: (startTime, endTime) => {
    const oldState = get();
    set({ startTime, endTime });
    const newState = get();
    newState._triggerSubscriptions(newState, oldState);
  },

  // Batch operations
  updateChartState: (updates) => {
    const oldState = get();
    set(updates);
    const newState = get();
    newState._triggerSubscriptions(newState, oldState);
  },

  resetToDefaults: () => {
    const oldState = get();
    set({ ...DEFAULT_CONFIG });
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
    const symbolChanged = newState.symbol !== oldState.symbol;
    const timeRangeChanged = newState.startTime !== oldState.startTime ||
      newState.endTime !== oldState.endTime;
    const presetChanged = newState.preset !== oldState.preset;

    // Trigger specific callbacks
    newState._subscriptions?.forEach((callbacks: StoreSubscriptionCallbacks) => {
      if (symbolChanged && callbacks.onSymbolChange) {
        callbacks.onSymbolChange(newState.symbol, oldState.symbol);
      }

      if (timeRangeChanged && callbacks.onTimeRangeChange) {
        callbacks.onTimeRangeChange(
          { startTime: newState.startTime, endTime: newState.endTime },
          { startTime: oldState.startTime, endTime: oldState.endTime }
        );
      }

      // Preset changes - note the callback expects string[] but we have string
      if (presetChanged && callbacks.onPresetChange) {
        callbacks.onPresetChange(newState.preset, oldState.preset);
      }

      // Always trigger general change callback
      if (callbacks.onAnyChange) {
        const newStoreState: StoreState = {
          symbol: newState.symbol,
          preset: newState.preset,
          startTime: newState.startTime,
          endTime: newState.endTime
        };
        const oldStoreState: StoreState = {
          symbol: oldState.symbol,
          preset: oldState.preset,
          startTime: oldState.startTime,
          endTime: oldState.endTime
        };
        callbacks.onAnyChange(newStoreState, oldStoreState);
      }
    });

    // Update last state reference using a partial update that only affects AppStore properties
    set((state) => ({
      ...state,
      _lastState: {
        symbol: newState.symbol,
        preset: newState.preset,
        startTime: newState.startTime,
        endTime: newState.endTime
      }
    }));
  },
}));

// Export helper hooks for specific subscriptions
export const useSymbolSubscription = (callback: (newSymbol?: string, oldSymbol?: string) => void) => {
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
