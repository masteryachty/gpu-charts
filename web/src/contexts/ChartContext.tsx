import React, { createContext, useContext, useRef, useState, ReactNode } from 'react';
import { Chart } from '@pkg/wasm_bridge.js';
import { useChartStore } from '../store/useChartStore';

/**
 * Chart Context for eliminating prop drilling
 * Provides chart instance and related state throughout the component tree
 */
interface ChartContextType {
  // Chart instance
  chartInstance: Chart | null;
  setChartInstance: (chart: Chart | null) => void;
  
  // Chart initialization state
  isInitialized: boolean;
  setIsInitialized: (initialized: boolean) => void;
  
  // Applied preset (after WASM processing)
  appliedPreset?: string;
  setAppliedPreset: (preset?: string) => void;
  
  // Chart canvas reference
  canvasRef: React.RefObject<HTMLCanvasElement>;
  
  // Chart configuration
  chartConfig: ChartConfig;
  setChartConfig: (config: Partial<ChartConfig>) => void;
  
  // Chart operations
  applyPreset: (presetName: string) => Promise<void>;
  resetChart: () => void;
  renderChart: () => Promise<void>;
}

interface ChartConfig {
  width: number;
  height: number;
  canvasId: string;
  quality: 'low' | 'medium' | 'high' | 'ultra';
}

const defaultChartConfig: ChartConfig = {
  width: 800,
  height: 600,
  canvasId: 'webgpu-canvas',
  quality: 'high',
};

const ChartContext = createContext<ChartContextType | null>(null);

/**
 * Chart Context Provider
 * Manages chart instance and provides it to child components
 */
interface ChartProviderProps {
  children: ReactNode;
  initialConfig?: Partial<ChartConfig>;
}

export function ChartProvider({ children, initialConfig }: ChartProviderProps) {
  const [chartInstance, setChartInstance] = useState<Chart | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);
  const [appliedPreset, setAppliedPreset] = useState<string>();
  const [chartConfig, setChartConfigState] = useState<ChartConfig>({
    ...defaultChartConfig,
    ...initialConfig,
  });
  
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const chartStore = useChartStore();

  const setChartConfig = (config: Partial<ChartConfig>) => {
    setChartConfigState(prev => ({ ...prev, ...config }));
  };

  const applyPreset = async (presetName: string) => {
    if (!chartInstance) {
      throw new Error('Chart instance not available');
    }

    try {
      // Use the WASM method if available
      if (chartInstance.apply_preset_and_symbols) {
        await chartInstance.apply_preset_and_symbols(presetName, [chartStore.symbol || 'BTC-USD']);
      } else if (chartInstance.apply_preset) {
        await chartInstance.apply_preset(presetName);
      }
      
      setAppliedPreset(presetName);
    } catch (error) {
      console.error('[ChartContext] Failed to apply preset:', error);
      throw error;
    }
  };

  const resetChart = () => {
    setChartInstance(null);
    setIsInitialized(false);
    setAppliedPreset(undefined);
    chartStore.resetChartState();
  };

  const renderChart = async () => {
    if (!chartInstance || !isInitialized) {
      throw new Error('Chart not initialized');
    }

    try {
      if (chartInstance.render) {
        await chartInstance.render();
      }
    } catch (error) {
      console.error('[ChartContext] Failed to render chart:', error);
      throw error;
    }
  };

  const contextValue: ChartContextType = {
    chartInstance,
    setChartInstance,
    isInitialized,
    setIsInitialized,
    appliedPreset,
    setAppliedPreset,
    canvasRef,
    chartConfig,
    setChartConfig,
    applyPreset,
    resetChart,
    renderChart,
  };

  return (
    <ChartContext.Provider value={contextValue}>
      {children}
    </ChartContext.Provider>
  );
}

/**
 * Hook to access chart context
 * Throws error if used outside ChartProvider
 */
export function useChartContext(): ChartContextType {
  const context = useContext(ChartContext);
  if (!context) {
    throw new Error('useChartContext must be used within a ChartProvider');
  }
  return context;
}

/**
 * Hook to access chart instance safely
 * Returns null if chart not available
 */
export function useChart(): Chart | null {
  const context = useContext(ChartContext);
  return context?.chartInstance || null;
}

/**
 * Hook to check if chart is ready for operations
 */
export function useChartReady(): boolean {
  const context = useContext(ChartContext);
  return !!(context?.chartInstance && context.isInitialized);
}