import { create } from 'zustand';

/**
 * Market data state store
 * Handles data fetching, caching, and real-time updates
 */
export interface MarketDataState {
  // Data loading states
  isLoading?: boolean;
  error?: string;
  lastUpdate?: number;
  
  // Data cache
  dataCache?: Map<string, any>;
  
  // Real-time data
  realtimeEnabled?: boolean;
  connectionStatus?: 'connecting' | 'connected' | 'disconnected' | 'error';
  
  // Data statistics
  totalDataPoints?: number;
  dataRange?: { start: number; end: number };
  
  // Available data
  availableSymbols?: string[];
  availableExchanges?: string[];
}

interface MarketDataStore extends MarketDataState {
  // Loading state actions
  setLoading: (loading: boolean) => void;
  setError: (error?: string) => void;
  setLastUpdate: (timestamp: number) => void;
  
  // Data actions
  setDataCache: (cache: Map<string, any>) => void;
  clearDataCache: () => void;
  addToDataCache: (key: string, data: any) => void;
  
  // Real-time actions
  setRealtimeEnabled: (enabled: boolean) => void;
  setConnectionStatus: (status: 'connecting' | 'connected' | 'disconnected' | 'error') => void;
  
  // Data statistics actions
  setTotalDataPoints: (count: number) => void;
  setDataRange: (start: number, end: number) => void;
  
  // Available data actions
  setAvailableSymbols: (symbols: string[]) => void;
  setAvailableExchanges: (exchanges: string[]) => void;
  
  // Utility actions
  resetMarketDataState: () => void;
  updateMarketDataState: (updates: Partial<MarketDataState>) => void;
}

const defaultMarketDataState: MarketDataState = {
  isLoading: false,
  error: undefined,
  lastUpdate: undefined,
  dataCache: new Map(),
  realtimeEnabled: false,
  connectionStatus: 'disconnected',
  totalDataPoints: 0,
  dataRange: undefined,
  availableSymbols: [],
  availableExchanges: [],
};

export const useMarketDataStore = create<MarketDataStore>()((set, get) => ({
  ...defaultMarketDataState,

  setLoading: (loading: boolean) => {
    set({ isLoading: loading });
    // Clear error when starting to load
    if (loading) {
      set({ error: undefined });
    }
  },

  setError: (error?: string) => {
    set({ error, isLoading: false });
  },

  setLastUpdate: (timestamp: number) => {
    set({ lastUpdate: timestamp });
  },

  setDataCache: (cache: Map<string, any>) => {
    set({ dataCache: cache });
  },

  clearDataCache: () => {
    set({ dataCache: new Map() });
  },

  addToDataCache: (key: string, data: any) => {
    const currentCache = get().dataCache || new Map();
    const newCache = new Map(currentCache);
    newCache.set(key, data);
    set({ dataCache: newCache });
  },

  setRealtimeEnabled: (enabled: boolean) => {
    set({ realtimeEnabled: enabled });
    // Update connection status when enabling/disabling
    if (!enabled) {
      set({ connectionStatus: 'disconnected' });
    }
  },

  setConnectionStatus: (status: 'connecting' | 'connected' | 'disconnected' | 'error') => {
    set({ connectionStatus: status });
  },

  setTotalDataPoints: (count: number) => {
    set({ totalDataPoints: count });
  },

  setDataRange: (start: number, end: number) => {
    set({ dataRange: { start, end } });
  },

  setAvailableSymbols: (symbols: string[]) => {
    set({ availableSymbols: symbols });
  },

  setAvailableExchanges: (exchanges: string[]) => {
    set({ availableExchanges: exchanges });
  },

  resetMarketDataState: () => {
    set({ ...defaultMarketDataState, dataCache: new Map() });
  },

  updateMarketDataState: (updates: Partial<MarketDataState>) => {
    set(updates);
  },
}));