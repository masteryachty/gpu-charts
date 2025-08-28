import { useEffect, useState } from 'react';
import { useWasmInitialization } from '../../hooks/useWasmInitialization';
import { LoadingSkeleton } from './LoadingSkeleton';

interface AppLoadingScreenProps {
  onInitialized?: () => void;
  className?: string;
}

interface LoadingStep {
  id: string;
  label: string;
  description: string;
  completed: boolean;
  active: boolean;
}

export function AppLoadingScreen({ onInitialized, className = '' }: AppLoadingScreenProps) {
  const { 
    isInitialized, 
    isInitializing, 
    hasError, 
    error, 
    initializationProgress,
    retry 
  } = useWasmInitialization();

  const [steps, setSteps] = useState<LoadingStep[]>([
    {
      id: 'webgpu',
      label: 'GPU Acceleration',
      description: 'Checking WebGPU support',
      completed: false,
      active: false
    },
    {
      id: 'wasm',
      label: 'Chart Engine',
      description: 'Loading WebAssembly module',
      completed: false,
      active: false
    },
    {
      id: 'gpu-context',
      label: 'Rendering System',
      description: 'Initializing GPU context',
      completed: false,
      active: false
    },
    {
      id: 'finalize',
      label: 'Ready to Trade',
      description: 'Finalizing setup',
      completed: false,
      active: false
    }
  ]);

  // Update steps based on progress
  useEffect(() => {
    const progress = initializationProgress;
    
    setSteps(prev => prev.map((step, index) => {
      const stepProgress = (index + 1) * 25; // 25%, 50%, 75%, 100%
      const isCompleted = progress >= stepProgress;
      const isActive = progress >= (index * 25) && progress < stepProgress;
      
      return {
        ...step,
        completed: isCompleted,
        active: isActive && !isCompleted
      };
    }));
  }, [initializationProgress]);

  // Call onInitialized when ready
  useEffect(() => {
    if (isInitialized && onInitialized) {
      onInitialized();
    }
  }, [isInitialized, onInitialized]);

  if (isInitialized) {
    return null;
  }

  return (
    <div 
      className={`fixed inset-0 bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900 flex items-center justify-center z-50 ${className}`}
      role="status"
      aria-live="polite"
      aria-label="Application initializing"
    >
      <div className="max-w-md w-full mx-4">
        {/* Header */}
        <div className="text-center mb-8">
          <div className="text-4xl mb-4 animate-pulse">
            <span className="text-blue-500">⚡</span>
            <span className="text-green-500 ml-2">📈</span>
          </div>
          <h1 className="text-2xl font-bold text-white mb-2">GPU Charts</h1>
          <p className="text-gray-400">High-performance financial visualization</p>
        </div>

        {/* Progress Bar */}
        <div className="mb-8">
          <div className="flex items-center justify-between text-sm text-gray-400 mb-2">
            <span>Initializing...</span>
            <span>{Math.round(initializationProgress)}%</span>
          </div>
          <div className="w-full bg-gray-700 rounded-full h-3 overflow-hidden">
            <div 
              className="h-3 bg-gradient-to-r from-blue-500 to-green-500 transition-all duration-500 ease-out"
              style={{ width: `${initializationProgress}%` }}
              role="progressbar"
              aria-valuenow={initializationProgress}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-label={`Initialization progress: ${Math.round(initializationProgress)}%`}
            />
          </div>
        </div>

        {/* Loading Steps */}
        <div className="space-y-3 mb-8">
          {steps.map((step, index) => (
            <div 
              key={step.id}
              className={`flex items-center p-3 rounded-lg transition-all duration-300 ${
                step.completed 
                  ? 'bg-green-900/30 border-green-500/50 border' 
                  : step.active 
                    ? 'bg-blue-900/30 border-blue-500/50 border' 
                    : 'bg-gray-800/50'
              }`}
            >
              <div className="flex-shrink-0 mr-3">
                {step.completed ? (
                  <div className="w-5 h-5 rounded-full bg-green-500 flex items-center justify-center">
                    <svg className="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                    </svg>
                  </div>
                ) : step.active ? (
                  <div className="w-5 h-5 rounded-full border-2 border-blue-500 border-t-transparent animate-spin" />
                ) : (
                  <div className="w-5 h-5 rounded-full bg-gray-600" />
                )}
              </div>
              <div className="flex-1">
                <div className={`font-medium ${
                  step.completed ? 'text-green-400' : step.active ? 'text-blue-400' : 'text-gray-400'
                }`}>
                  {step.label}
                </div>
                <div className="text-sm text-gray-500">{step.description}</div>
              </div>
            </div>
          ))}
        </div>

        {/* Error State */}
        {hasError && error && (
          <div className="bg-red-900/30 border border-red-500/50 rounded-lg p-4 mb-4">
            <div className="flex items-center mb-2">
              <svg className="w-5 h-5 text-red-400 mr-2" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
              </svg>
              <span className="text-red-400 font-medium">Initialization Failed</span>
            </div>
            <p className="text-red-300 text-sm mb-3">{error.message}</p>
            <button
              onClick={retry}
              className="bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded font-medium transition-colors"
            >
              Retry
            </button>
          </div>
        )}

        {/* Loading Animation */}
        {isInitializing && !hasError && (
          <div className="text-center">
            <div className="inline-flex items-center gap-2 text-gray-400 text-sm">
              <div className="flex gap-1">
                {[...Array(3)].map((_, i) => (
                  <div 
                    key={i}
                    className="w-2 h-2 bg-blue-500 rounded-full animate-bounce"
                    style={{ animationDelay: `${i * 0.2}s` }}
                  />
                ))}
              </div>
              <span>Please wait...</span>
            </div>
          </div>
        )}

        {/* Hardware Requirements Notice */}
        <div className="mt-8 p-4 bg-gray-800/50 rounded-lg">
          <div className="text-xs text-gray-500 text-center">
            <p className="mb-1">🔧 Requires WebGPU support</p>
            <p>Chrome 113+ • Edge 113+ • Firefox 110+</p>
          </div>
        </div>
      </div>
    </div>
  );
}