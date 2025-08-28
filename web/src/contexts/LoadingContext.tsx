import { createContext, useContext, useState, useCallback, ReactNode } from 'react';

interface LoadingState {
  wasm: boolean;
  data: boolean;
  symbols: boolean;
  webgpu: boolean;
  initialization: boolean;
}

interface LoadingContextValue {
  loading: LoadingState;
  setLoading: (key: keyof LoadingState, value: boolean) => void;
  setMultipleLoading: (updates: Partial<LoadingState>) => void;
  isAnyLoading: boolean;
  getLoadingMessage: () => string;
}

const LoadingContext = createContext<LoadingContextValue | undefined>(undefined);

const LOADING_MESSAGES = {
  wasm: 'Loading WebAssembly module...',
  data: 'Fetching market data...',
  symbols: 'Loading available symbols...',
  webgpu: 'Initializing GPU acceleration...',
  initialization: 'Starting chart engine...',
};

const LOADING_PRIORITIES = {
  initialization: 5,
  webgpu: 4,
  wasm: 3,
  data: 2,
  symbols: 1,
};

interface LoadingProviderProps {
  children: ReactNode;
}

export function LoadingProvider({ children }: LoadingProviderProps) {
  const [loading, setLoadingState] = useState<LoadingState>({
    wasm: false,
    data: false,
    symbols: false,
    webgpu: false,
    initialization: false,
  });

  const setLoading = useCallback((key: keyof LoadingState, value: boolean) => {
    setLoadingState(prev => ({
      ...prev,
      [key]: value
    }));
  }, []);

  const setMultipleLoading = useCallback((updates: Partial<LoadingState>) => {
    setLoadingState(prev => ({
      ...prev,
      ...updates
    }));
  }, []);

  const isAnyLoading = Object.values(loading).some(Boolean);

  const getLoadingMessage = useCallback(() => {
    const activeLoadingStates = Object.entries(loading)
      .filter(([_, isLoading]) => isLoading)
      .map(([key]) => key as keyof LoadingState);

    if (activeLoadingStates.length === 0) {
      return '';
    }

    // Return the message for the highest priority loading state
    const highestPriority = activeLoadingStates.reduce((highest, current) => {
      return LOADING_PRIORITIES[current] > LOADING_PRIORITIES[highest] ? current : highest;
    });

    return LOADING_MESSAGES[highestPriority];
  }, [loading]);

  return (
    <LoadingContext.Provider value={{
      loading,
      setLoading,
      setMultipleLoading,
      isAnyLoading,
      getLoadingMessage,
    }}>
      {children}
    </LoadingContext.Provider>
  );
}

export function useLoading() {
  const context = useContext(LoadingContext);
  if (context === undefined) {
    throw new Error('useLoading must be used within a LoadingProvider');
  }
  return context;
}

interface GlobalLoadingOverlayProps {
  className?: string;
}

export function GlobalLoadingOverlay({ className = '' }: GlobalLoadingOverlayProps) {
  const { isAnyLoading, getLoadingMessage } = useLoading();

  if (!isAnyLoading) {
    return null;
  }

  return (
    <div 
      className={`fixed inset-0 bg-black/50 flex items-center justify-center z-[9999] ${className}`}
      role="status"
      aria-live="polite"
      aria-label="Application loading"
    >
      <div className="bg-gray-800 border border-gray-600 rounded-lg p-6 text-center min-w-[300px]">
        <div className="animate-spin text-blue-500 text-3xl mb-4" aria-hidden="true">⚡</div>
        <div className="text-white font-medium mb-2">GPU Charts</div>
        <div className="text-gray-400 text-sm mb-3">{getLoadingMessage()}</div>
        <div className="w-full bg-gray-700 rounded-full h-2">
          <div 
            className="bg-gradient-to-r from-blue-500 to-green-500 h-2 rounded-full animate-pulse"
            style={{ width: '60%' }}
            aria-hidden="true"
          />
        </div>
      </div>
    </div>
  );
}