import React from 'react';
import { ErrorBoundary, DefaultErrorFallback } from './ErrorBoundary';
import { AlertTriangle, RotateCcw, Monitor, Zap } from 'lucide-react';
import type { ErrorFallbackProps } from './ErrorBoundary';

/**
 * Specialized error boundary for WASM canvas and WebGPU operations
 * Provides specific recovery options for GPU-related failures
 */

interface WasmErrorFallbackProps extends ErrorFallbackProps {
  isWebGPUAvailable?: boolean;
  canvasId?: string;
}

function WasmErrorFallback({
  error,
  errorInfo,
  retryCount,
  isRecovering,
  onRetry,
  onReset,
  onReportError
}: WasmErrorFallbackProps) {
  const isWebGPUError = error.message.toLowerCase().includes('webgpu') || 
                        error.message.toLowerCase().includes('gpu') ||
                        error.message.toLowerCase().includes('adapter');
  
  const isWasmError = error.message.toLowerCase().includes('wasm') ||
                      error.message.toLowerCase().includes('webassembly');

  const getErrorType = () => {
    if (isWebGPUError) return 'WebGPU';
    if (isWasmError) return 'WebAssembly';
    return 'Chart Rendering';
  };

  const getErrorIcon = () => {
    if (isWebGPUError) return <Monitor className="h-8 w-8 text-red-400" />;
    if (isWasmError) return <Zap className="h-8 w-8 text-yellow-400" />;
    return <AlertTriangle className="h-8 w-8 text-red-400" />;
  };

  const getErrorDescription = () => {
    if (isWebGPUError) {
      return 'The GPU acceleration engine failed to initialize or encountered an error during rendering.';
    }
    if (isWasmError) {
      return 'The WebAssembly chart engine failed to load or execute properly.';
    }
    return 'An error occurred while rendering the chart canvas.';
  };

  const getSuggestedActions = () => {
    const actions = [];
    
    if (isWebGPUError) {
      actions.push('Check if your browser supports WebGPU');
      actions.push('Update your graphics drivers');
      actions.push('Try a different browser (Chrome 113+, Firefox 110+)');
      actions.push('Disable hardware acceleration if issues persist');
    } else if (isWasmError) {
      actions.push('Ensure WebAssembly is enabled in your browser');
      actions.push('Clear your browser cache and reload');
      actions.push('Try a different browser');
    } else {
      actions.push('Refresh the page to reinitialize the chart');
      actions.push('Check your internet connection');
      actions.push('Try clearing browser cache');
    }
    
    return actions;
  };

  const handleFallbackMode = () => {
    // Enable software fallback mode
    (window as any).__FORCE_SOFTWARE_RENDERING__ = true;
    (window as any).__DISABLE_WEBGPU__ = true;
    onRetry();
  };

  return (
    <div className="flex-1 flex items-center justify-center bg-gray-900 p-6">
      <div className="max-w-2xl w-full bg-gray-800 border border-red-500/30 rounded-lg p-8">
        {/* Header */}
        <div className="flex items-center mb-6">
          <div className="mr-4">
            {getErrorIcon()}
          </div>
          <div>
            <h1 className="text-2xl font-bold text-white mb-1">
              {getErrorType()} Error
            </h1>
            <p className="text-gray-400">{getErrorDescription()}</p>
          </div>
        </div>

        {/* Error Details */}
        <div className="mb-6 space-y-4">
          <div className="bg-red-950/30 border border-red-500/20 rounded p-4">
            <div className="flex items-start gap-3">
              <AlertTriangle className="h-5 w-5 text-red-400 mt-0.5 flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-red-300 font-medium mb-2">
                  {error.name}: {error.message}
                </p>
                <p className="text-red-400/80 text-sm">
                  Retry attempts: {retryCount} / 3
                </p>
              </div>
            </div>
          </div>

          {/* Browser Compatibility Info */}
          {isWebGPUError && (
            <div className="bg-yellow-950/30 border border-yellow-500/20 rounded p-4">
              <h4 className="text-yellow-300 font-medium mb-2 flex items-center gap-2">
                <Monitor className="h-4 w-4" />
                WebGPU Compatibility
              </h4>
              <div className="text-yellow-300/80 text-sm space-y-1">
                <p>• Chrome 113+ or Firefox 110+ required</p>
                <p>• Hardware acceleration must be enabled</p>
                <p>• Modern graphics drivers recommended</p>
              </div>
            </div>
          )}

          {/* Suggested Actions */}
          <div className="bg-blue-950/30 border border-blue-500/20 rounded p-4">
            <h4 className="text-blue-300 font-medium mb-3">Suggested Actions:</h4>
            <ul className="text-blue-300/80 text-sm space-y-2">
              {getSuggestedActions().map((action, index) => (
                <li key={index} className="flex items-start gap-2">
                  <span className="text-blue-400 mt-1">•</span>
                  <span>{action}</span>
                </li>
              ))}
            </ul>
          </div>

          {/* Technical Details */}
          <details className="bg-gray-900 rounded p-4">
            <summary className="cursor-pointer text-gray-300 hover:text-white flex items-center gap-2">
              <span>🔧</span> Technical Details
            </summary>
            <div className="mt-3 space-y-2">
              <div className="text-sm">
                <span className="text-gray-400">Error Stack:</span>
                <pre className="mt-1 text-xs text-gray-500 overflow-auto whitespace-pre-wrap bg-gray-950 p-2 rounded">
                  {error.stack}
                </pre>
              </div>
              {errorInfo && (
                <div className="text-sm">
                  <span className="text-gray-400">Component Stack:</span>
                  <pre className="mt-1 text-xs text-gray-500 overflow-auto whitespace-pre-wrap bg-gray-950 p-2 rounded">
                    {errorInfo.componentStack}
                  </pre>
                </div>
              )}
            </div>
          </details>
        </div>

        {/* Action Buttons */}
        <div className="flex flex-wrap gap-3">
          <button
            onClick={onRetry}
            disabled={isRecovering || retryCount >= 3}
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <RotateCcw className="h-4 w-4" />
            {isRecovering ? 'Retrying...' : 'Retry'}
          </button>

          {isWebGPUError && (
            <button
              onClick={handleFallbackMode}
              className="flex items-center gap-2 px-4 py-2 bg-yellow-600 text-white rounded hover:bg-yellow-700 transition-colors"
            >
              <Monitor className="h-4 w-4" />
              Try Software Mode
            </button>
          )}

          <button
            onClick={onReset}
            className="flex items-center gap-2 px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700 transition-colors"
          >
            <RotateCcw className="h-4 w-4" />
            Reset Chart
          </button>

          <button
            onClick={() => window.location.reload()}
            className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors"
          >
            <RotateCcw className="h-4 w-4" />
            Reload Page
          </button>

          <button
            onClick={onReportError}
            className="flex items-center gap-2 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 transition-colors"
          >
            🐛 Report Error
          </button>
        </div>
      </div>
    </div>
  );
}

export interface WasmErrorBoundaryProps {
  children: React.ReactNode;
  canvasId?: string;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
}

export function WasmErrorBoundary({
  children,
  canvasId,
  onError
}: WasmErrorBoundaryProps) {
  const handleError = (error: Error, errorInfo: React.ErrorInfo) => {
    // Log WebGPU/WASM specific error details
    console.group(`🎮 WASM/WebGPU Error in ${canvasId || 'chart canvas'}`);
    console.error('Canvas ID:', canvasId);
    console.error('WebGPU Available:', 'gpu' in navigator);
    console.error('WASM Supported:', 'WebAssembly' in window);
    console.error('Error:', error);
    console.error('Component Stack:', errorInfo.componentStack);
    console.groupEnd();

    // Check browser capabilities
    const capabilities = {
      webgpu: 'gpu' in navigator,
      wasm: 'WebAssembly' in window,
      canvas: !!document.createElement('canvas').getContext('2d'),
      userAgent: navigator.userAgent
    };

    console.info('Browser Capabilities:', capabilities);

    if (onError) {
      onError(error, errorInfo);
    }
  };

  return (
    <ErrorBoundary
      onError={handleError}
      componentName={`WASM Canvas ${canvasId || ''}`}
      enableReporting={true}
      enableAutoRecovery={true}
      maxRetryAttempts={3}
      fallback={WasmErrorFallback}
    >
      {children}
    </ErrorBoundary>
  );
}

export default WasmErrorBoundary;