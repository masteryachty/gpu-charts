import { create } from 'zustand';
import { AppState, ChartConfig, MarketData } from '../types';

interface AppStore extends AppState {
  // Actions
  setCurrentSymbol: (symbol: string) => void;
  setChartConfig: (config: ChartConfig) => void;
  updateMarketData: (symbol: string, data: MarketData) => void;
  setConnectionStatus: (connected: boolean) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  // Initial state
  currentSymbol: 'BTC-usd',
  chartConfig: {
    symbol: 'BTC-usd',
    timeframe: '1D',
    startTime: 1745460900, // 24 hours ago
    endTime: 1745553000,
    indicators: [],
  },
  marketData: {},
  isConnected: false,
  user: undefined,

  // Actions
  setCurrentSymbol: (symbol) => 
    set((state) => ({
      currentSymbol: symbol,
      chartConfig: { ...state.chartConfig, symbol },
    })),

  setChartConfig: (config) => 
    set({ chartConfig: config }),

  updateMarketData: (symbol, data) =>
    set((state) => ({
      marketData: {
        ...state.marketData,
        [symbol]: data,
      },
    })),

  setConnectionStatus: (connected) =>
    set({ isConnected: connected }),
}));