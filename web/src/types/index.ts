// Market data types
export interface MarketData {
  symbol: string;
  price: number;
  change: number;
  changePercent: number;
  volume: number;
  timestamp: number;
}

// Chart configuration
export interface ChartConfig {
  symbol: string;
  timeframe: string;
  startTime: number;
  endTime: number;
  indicators: string[];
}

// WASM integration types
export interface WasmModule {
  memory: WebAssembly.Memory;
  // Add other WASM exports as needed
}

// Application state
export interface AppState {
  currentSymbol: string;
  chartConfig: ChartConfig;
  marketData: Record<string, MarketData>;
  isConnected: boolean;
  user?: User;
}

export interface User {
  id: string;
  name: string;
  email: string;
  plan: 'free' | 'pro' | 'enterprise';
}

// Navigation
export interface NavItem {
  label: string;
  href: string;
  icon?: React.ComponentType;
}

// Performance metrics
export interface PerformanceMetrics {
  fps: number;
  frameTime: number;
  latency: number;
}