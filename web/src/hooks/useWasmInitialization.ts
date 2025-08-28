import { useState, useEffect, useCallback } from 'react';
import { useLoading } from '../contexts/LoadingContext';

interface WasmInitializationState {
  isInitialized: boolean;
  isInitializing: boolean;
  hasError: boolean;
  error: Error | null;
  initializationProgress: number;
}

interface WasmInitializationHook extends WasmInitializationState {
  initialize: () => Promise<boolean>;
  retry: () => Promise<boolean>;
  reset: () => void;
}

interface InitializationStep {
  name: string;
  description: string;
  execute: () => Promise<void>;
}

export function useWasmInitialization(): WasmInitializationHook {
  const { setLoading, setMultipleLoading } = useLoading();
  
  const [state, setState] = useState<WasmInitializationState>({
    isInitialized: false,
    isInitializing: false,
    hasError: false,
    error: null,
    initializationProgress: 0,
  });

  const updateProgress = useCallback((progress: number) => {
    setState(prev => ({ ...prev, initializationProgress: progress }));
  }, []);

  const setError = useCallback((error: Error) => {
    setState(prev => ({ 
      ...prev, 
      hasError: true, 
      error, 
      isInitializing: false 
    }));
    setMultipleLoading({ 
      initialization: false, 
      wasm: false, 
      webgpu: false 
    });
  }, [setMultipleLoading]);

  const initialize = useCallback(async (): Promise<boolean> => {
    if (state.isInitialized || state.isInitializing) {
      return state.isInitialized;
    }

    setState(prev => ({ 
      ...prev, 
      isInitializing: true, 
      hasError: false, 
      error: null, 
      initializationProgress: 0 
    }));

    setLoading('initialization', true);

    try {
      const steps: InitializationStep[] = [
        {
          name: 'webgpu-check',
          description: 'Checking WebGPU availability',
          execute: async () => {
            updateProgress(10);
            setLoading('webgpu', true);
            
            if (!('gpu' in navigator)) {
              throw new Error('WebGPU is not supported in this browser');
            }
            
            // Test WebGPU adapter request
            const adapter = await (navigator as any).gpu.requestAdapter();
            if (!adapter) {
              throw new Error('Failed to get WebGPU adapter');
            }
            
            updateProgress(25);
          }
        },
        {
          name: 'wasm-load',
          description: 'Loading WebAssembly module',
          execute: async () => {
            setLoading('wasm', true);
            updateProgress(40);
            
            // Dynamic import of the WASM module
            try {
              // Check if WASM module is available
              const wasmModule = await import('../../pkg');
              updateProgress(60);
              
              // Initialize the module if it has an init function
              if (wasmModule.default) {
                await wasmModule.default();
              }
              
              updateProgress(75);
            } catch (error) {
              throw new Error(`Failed to load WebAssembly module: ${error instanceof Error ? error.message : 'Unknown error'}`);
            }
          }
        },
        {
          name: 'gpu-context',
          description: 'Initializing GPU context',
          execute: async () => {
            updateProgress(80);
            
            // Additional GPU context setup if needed
            const canvas = document.createElement('canvas');
            const context = canvas.getContext('webgpu');
            if (!context) {
              throw new Error('Failed to get WebGPU context');
            }
            
            updateProgress(90);
          }
        },
        {
          name: 'finalize',
          description: 'Finalizing initialization',
          execute: async () => {
            updateProgress(95);
            
            // Small delay to ensure all systems are ready
            await new Promise(resolve => setTimeout(resolve, 100));
            
            updateProgress(100);
          }
        }
      ];

      for (const step of steps) {
        try {
          await step.execute();
        } catch (error) {
          throw new Error(`Initialization failed at ${step.name}: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
      }

      setState(prev => ({ 
        ...prev, 
        isInitialized: true, 
        isInitializing: false, 
        initializationProgress: 100 
      }));

      setMultipleLoading({ 
        initialization: false, 
        wasm: false, 
        webgpu: false 
      });

      return true;

    } catch (error) {
      const err = error instanceof Error ? error : new Error('Unknown initialization error');
      console.error('[WASM Initialization] Failed:', err);
      setError(err);
      return false;
    }
  }, [state.isInitialized, state.isInitializing, setLoading, setMultipleLoading, updateProgress, setError]);

  const retry = useCallback(async (): Promise<boolean> => {
    setState(prev => ({ 
      ...prev, 
      hasError: false, 
      error: null, 
      isInitialized: false,
      initializationProgress: 0
    }));
    return initialize();
  }, [initialize]);

  const reset = useCallback(() => {
    setState({
      isInitialized: false,
      isInitializing: false,
      hasError: false,
      error: null,
      initializationProgress: 0,
    });
    setMultipleLoading({ 
      initialization: false, 
      wasm: false, 
      webgpu: false 
    });
  }, [setMultipleLoading]);

  // Auto-initialize on mount
  useEffect(() => {
    if (!state.isInitialized && !state.isInitializing && !state.hasError) {
      initialize();
    }
  }, [initialize, state.isInitialized, state.isInitializing, state.hasError]);

  return {
    ...state,
    initialize,
    retry,
    reset,
  };
}