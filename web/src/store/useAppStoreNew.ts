/**
 * Compatibility layer for the new separated stores
 * Provides the same interface as the old useAppStore but uses the new domain stores
 * This makes migration easier while maintaining backward compatibility
 */
import { useChartStore, ChartState } from './useChartStore';
import { useUIStore, UIState } from './useUIStore';
import { useMarketDataStore, MarketDataState } from './useMarketDataStore';
import { useMemo } from 'react';

// Legacy interface for backward compatibility
export interface StoreState {
  preset?: string;
  symbol?: string;
  startTime: number;
  endTime: number;
  isConnected?: boolean;
  comparisonMode?: boolean;
  selectedExchanges?: string[];
  baseSymbol?: string;
}

// Store subscription callback interface (keeping for compatibility)
export interface StoreSubscriptionCallbacks {
  onSymbolChange?: (newSymbol?: string, oldSymbol?: string) => void;
  onTimeRangeChange?: (newRange: { startTime: number; endTime: number }, oldRange: { startTime: number; endTime: number }) => void;
  onPresetChange?: (newPreset?: string, oldPreset?: string) => void;
  onAnyChange?: (newState: StoreState, oldState: StoreState) => void;
}

/**
 * Compatibility hook that combines all domain stores
 * Drop-in replacement for the old useAppStore
 */
export function useAppStore() {
  const chartStore = useChartStore();
  const uiStore = useUIStore();
  const marketDataStore = useMarketDataStore();

  return useMemo(() => ({
    // Chart state (from useChartStore)
    symbol: chartStore.symbol,
    startTime: chartStore.startTime,
    endTime: chartStore.endTime,
    baseSymbol: chartStore.baseSymbol,
    comparisonMode: chartStore.comparisonMode,
    selectedExchanges: chartStore.selectedExchanges,
    isConnected: chartStore.isConnected,

    // UI state (from useUIStore)
    preset: uiStore.preset,
    theme: uiStore.theme,
    sidebarCollapsed: uiStore.sidebarCollapsed,

    // Market data state (from useMarketDataStore)
    isLoading: marketDataStore.isLoading,
    error: marketDataStore.error,
    connectionStatus: marketDataStore.connectionStatus,

    // Chart actions
    setCurrentSymbol: chartStore.setCurrentSymbol,
    setTimeRange: chartStore.setTimeRange,
    setBaseSymbol: chartStore.setBaseSymbol,
    setIsConnected: chartStore.setIsConnected,
    setComparisonMode: chartStore.setComparisonMode,
    toggleExchange: chartStore.toggleExchange,
    setSelectedExchanges: chartStore.setSelectedExchanges,

    // UI actions
    setPreset: uiStore.setPreset,
    setTheme: uiStore.setTheme,
    toggleTheme: uiStore.toggleTheme,
    setSidebarCollapsed: uiStore.setSidebarCollapsed,
    toggleSidebar: uiStore.toggleSidebar,

    // Market data actions
    setLoading: marketDataStore.setLoading,
    setError: marketDataStore.setError,
    setConnectionStatus: marketDataStore.setConnectionStatus,

    // Combined actions
    resetToDefaults: () => {
      chartStore.resetChartState();
      uiStore.resetUIState();
      marketDataStore.resetMarketDataState();
    },

    updateChartState: chartStore.updateChartState,

    // Legacy subscription system (simplified for compatibility)
    subscribe: () => {
      // This would need to be implemented if the subscription system is actually used
      console.warn('Store subscription system not yet implemented in new architecture');
      return () => {}; // Return empty unsubscribe function
    },

  }), [chartStore, uiStore, marketDataStore]);
}

// Export individual stores for direct usage
export { useChartStore, useUIStore, useMarketDataStore };

// Legacy subscription hook (for compatibility)
export function useChartSubscription(callbacks: StoreSubscriptionCallbacks) {
  return {
    subscribe: () => {
      console.warn('Chart subscription not yet implemented in new architecture');
      return () => {};
    }
  };
}