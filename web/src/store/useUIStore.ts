import { create } from 'zustand';

/**
 * UI-specific state store
 * Handles UI preferences, themes, layouts, and user settings
 */
export interface UIState {
  // Theme and appearance
  theme?: 'light' | 'dark';
  sidebarCollapsed?: boolean;
  
  // Chart UI preferences  
  preset?: string;
  showGrid?: boolean;
  showTooltips?: boolean;
  showLegend?: boolean;
  
  // Layout preferences
  chartHeight?: number;
  controlsWidth?: number;
  
  // User preferences
  autoRefresh?: boolean;
  refreshInterval?: number;
}

interface UIStore extends UIState {
  // Theme actions
  setTheme: (theme: 'light' | 'dark') => void;
  toggleTheme: () => void;
  
  // Layout actions
  setSidebarCollapsed: (collapsed: boolean) => void;
  toggleSidebar: () => void;
  
  // Chart UI actions
  setPreset: (preset?: string) => void;
  setShowGrid: (show: boolean) => void;
  setShowTooltips: (show: boolean) => void;
  setShowLegend: (show: boolean) => void;
  
  // Layout actions
  setChartHeight: (height: number) => void;
  setControlsWidth: (width: number) => void;
  
  // User preference actions
  setAutoRefresh: (enabled: boolean) => void;
  setRefreshInterval: (interval: number) => void;
  
  // Utility actions
  resetUIState: () => void;
  updateUIState: (updates: Partial<UIState>) => void;
}

const defaultUIState: UIState = {
  theme: 'dark',
  sidebarCollapsed: false,
  preset: undefined,
  showGrid: true,
  showTooltips: true,
  showLegend: true,
  chartHeight: 600,
  controlsWidth: 320,
  autoRefresh: false,
  refreshInterval: 5000, // 5 seconds
};

export const useUIStore = create<UIStore>()((set, get) => ({
  ...defaultUIState,

  setTheme: (theme: 'light' | 'dark') => {
    set({ theme });
  },

  toggleTheme: () => {
    const currentTheme = get().theme;
    set({ theme: currentTheme === 'dark' ? 'light' : 'dark' });
  },

  setSidebarCollapsed: (collapsed: boolean) => {
    set({ sidebarCollapsed: collapsed });
  },

  toggleSidebar: () => {
    set({ sidebarCollapsed: !get().sidebarCollapsed });
  },

  setPreset: (preset?: string) => {
    set({ preset });
  },

  setShowGrid: (show: boolean) => {
    set({ showGrid: show });
  },

  setShowTooltips: (show: boolean) => {
    set({ showTooltips: show });
  },

  setShowLegend: (show: boolean) => {
    set({ showLegend: show });
  },

  setChartHeight: (height: number) => {
    set({ chartHeight: height });
  },

  setControlsWidth: (width: number) => {
    set({ controlsWidth: width });
  },

  setAutoRefresh: (enabled: boolean) => {
    set({ autoRefresh: enabled });
  },

  setRefreshInterval: (interval: number) => {
    set({ refreshInterval: interval });
  },

  resetUIState: () => {
    set(defaultUIState);
  },

  updateUIState: (updates: Partial<UIState>) => {
    set(updates);
  },
}));