import { create } from 'zustand';

/**
 * Chart-specific state store
 * Handles chart data, symbols, time ranges, and comparison mode
 */
export interface ChartState {
  // Core chart data
  symbol?: string;
  startTime: number;
  endTime: number;
  baseSymbol?: string; // Base symbol without exchange prefix (e.g., "BTC-USD")
  
  // Multi-exchange comparison
  comparisonMode?: boolean;
  selectedExchanges?: string[];
  
  // Connection status
  isConnected?: boolean;
}

interface ChartStore extends ChartState {
  // Core chart actions
  setCurrentSymbol: (symbol: string) => void;
  setTimeRange: (startTime: number, endTime: number) => void;
  setBaseSymbol: (symbol: string) => void;
  setIsConnected: (connected: boolean) => void;
  
  // Comparison mode actions
  setComparisonMode: (enabled: boolean) => void;
  toggleExchange: (exchange: string, symbol: string) => void;
  setSelectedExchanges: (exchanges: string[]) => void;
  
  // Utility actions
  resetChartState: () => void;
  updateChartState: (updates: Partial<ChartState>) => void;
}

const defaultChartState: ChartState = {
  symbol: 'BTC-USD',
  startTime: Math.floor(Date.now() / 1000) - 86400, // 24 hours ago
  endTime: Math.floor(Date.now() / 1000),
  baseSymbol: 'BTC-USD',
  comparisonMode: false,
  selectedExchanges: [],
  isConnected: false,
};

export const useChartStore = create<ChartStore>()((set, get) => ({
  ...defaultChartState,

  setCurrentSymbol: (symbol: string) => {
    set({ symbol });
  },

  setTimeRange: (startTime: number, endTime: number) => {
    set({ startTime, endTime });
  },

  setBaseSymbol: (symbol: string) => {
    set({ baseSymbol: symbol });
  },

  setIsConnected: (connected: boolean) => {
    set({ isConnected: connected });
  },

  setComparisonMode: (enabled: boolean) => {
    const state = get();
    set({ comparisonMode: enabled });
    
    // Clear selected exchanges when disabling comparison mode
    if (!enabled) {
      set({ selectedExchanges: [] });
    }
  },

  toggleExchange: (exchange: string, symbol: string) => {
    const state = get();
    const exchangeSymbolId = `${exchange}:${symbol}`;
    const currentSelected = state.selectedExchanges || [];
    
    if (state.comparisonMode) {
      // In comparison mode, toggle selection (max 2)
      if (currentSelected.includes(exchangeSymbolId)) {
        set({ 
          selectedExchanges: currentSelected.filter(id => id !== exchangeSymbolId) 
        });
      } else if (currentSelected.length < 2) {
        set({ 
          selectedExchanges: [...currentSelected, exchangeSymbolId] 
        });
      }
    } else {
      // In single mode, replace selection
      set({ selectedExchanges: [exchangeSymbolId] });
    }
  },

  setSelectedExchanges: (exchanges: string[]) => {
    set({ selectedExchanges: exchanges });
  },

  resetChartState: () => {
    set(defaultChartState);
  },

  updateChartState: (updates: Partial<ChartState>) => {
    set(updates);
  },
}));