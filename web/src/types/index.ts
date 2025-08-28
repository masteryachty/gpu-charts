// Core types for the application

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

export interface ChartState {
  isInitialized: boolean;
  isLoading: boolean;
  error: Error | null;
  lastUpdate: number;
}

export interface MarketData {
  time: number;
  price: number;
  volume?: number;
  best_bid?: number;
  best_ask?: number;
  side?: number;
}

export interface PerformanceMetrics {
  fps: number;
  renderTime: number;
  dataPoints: number;
  memoryUsage: number;
}

export interface TooltipData {
  x: number;
  y: number;
  timestamp: number;
  price: number;
  volume?: number;
  exchange?: string;
  symbol?: string;
  change24h?: number;
}

export interface TourStep {
  id: string;
  title: string;
  content: string;
  target: string;
  placement: 'top' | 'bottom' | 'left' | 'right' | 'center';
  showProgress?: boolean;
  isOptional?: boolean;
}

export interface Tour {
  id: string;
  name: string;
  description?: string;
  steps: TourStep[];
}

// Re-export global types
export * from './global';