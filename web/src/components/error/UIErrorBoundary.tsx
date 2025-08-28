import React from 'react';
import { ErrorBoundary } from './ErrorBoundary';
import { Layout, RefreshCw, AlertTriangle, Settings } from 'lucide-react';
import type { ErrorFallbackProps } from './ErrorBoundary';

/**
 * Specialized error boundary for UI components and interactions
 * Provides graceful degradation for component failures
 */

function UIErrorFallback({
  error,
  errorInfo,
  retryCount,
  isRecovering,
  onRetry,
  onReset,
  onReportError
}: ErrorFallbackProps) {
  const isRenderError = error.message.toLowerCase().includes('render') ||
                        error.message.toLowerCase().includes('hook') ||
                        errorInfo?.componentStack.includes('Hook');
  
  const isStateError = error.message.toLowerCase().includes('state') ||
                       error.message.toLowerCase().includes('store') ||
                       error.message.toLowerCase().includes('zustand');

  const isEventError = error.message.toLowerCase().includes('event') ||
                       error.message.toLowerCase().includes('handler') ||
                       error.message.toLowerCase().includes('callback');

  const getErrorType = () => {
    if (isRenderError) return 'Component Rendering';
    if (isStateError) return 'State Management';
    if (isEventError) return 'Event Handling';
    return 'UI Component';
  };

  const getErrorIcon = () => {
    if (isRenderError) return <Layout className="h-8 w-8 text-red-400" />;
    if (isStateError) return <Settings className="h-8 w-8 text-orange-400" />;
    if (isEventError) return <AlertTriangle className="h-8 w-8 text-yellow-400" />;
    return <Layout className="h-8 w-8 text-red-400" />;
  };

  const getErrorDescription = () => {
    if (isRenderError) {
      return 'A component failed to render properly. This might be due to invalid data or a React hook issue.';
    }
    if (isStateError) {
      return 'An error occurred in the application state management. Some features may not work correctly.';
    }
    if (isEventError) {
      return 'An event handler encountered an error. User interactions may be affected.';
    }
    return 'A user interface component encountered an error.';
  };

  const getSuggestedActions = () => {
    const actions = [];
    
    if (isRenderError) {
      actions.push('Try refreshing the component');
      actions.push('Check if you have the latest browser version');
      actions.push('Disable browser extensions temporarily');
      actions.push('Clear React DevTools cache if installed');
    } else if (isStateError) {
      actions.push('Reset the application state');
      actions.push('Reload the page to reinitialize state');
      actions.push('Check for conflicting browser tabs');
    } else if (isEventError) {
      actions.push('Try the action again');
      actions.push('Use keyboard shortcuts as alternative');
      actions.push('Refresh to reset event handlers');
    } else {
      actions.push('Refresh the affected component');
      actions.push('Try using a different part of the application');
      actions.push('Reload the page if issues persist');
    }
    
    return actions;
  };

  const getComponentName = () => {
    const stack = errorInfo?.componentStack || '';
    const matches = stack.match(/in (\w+)/);
    return matches ? matches[1] : 'Unknown Component';
  };

  const handleResetState = () => {
    // Clear Zustand stores
    localStorage.removeItem('chart-store');
    localStorage.removeItem('ui-store');
    localStorage.removeItem('market-data-store');
    
    // Clear React state
    onReset();
  };

  return (
    <div className="flex items-center justify-center bg-gray-900 p-4 min-h-[200px]">
      <div className="max-w-lg w-full bg-gray-800 border border-orange-500/30 rounded-lg p-6">
        {/* Header */}
        <div className="flex items-center mb-4">
          <div className="mr-3">
            {getErrorIcon()}
          </div>
          <div>
            <h2 className="text-xl font-bold text-white mb-1">
              {getErrorType()} Error
            </h2>
            <p className="text-gray-400 text-sm">{getErrorDescription()}</p>
          </div>
        </div>

        {/* Error Details */}
        <div className="mb-4 space-y-3">
          <div className="bg-orange-950/30 border border-orange-500/20 rounded p-3">
            <div className="flex items-start gap-2">
              <AlertTriangle className="h-4 w-4 text-orange-400 mt-0.5 flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-orange-300 text-sm font-medium mb-1">
                  Error in {getComponentName()}
                </p>
                <p className="text-orange-400/80 text-xs">
                  {error.message}
                </p>
                <p className="text-orange-400/60 text-xs mt-1">
                  Attempts: {retryCount} / 3
                </p>
              </div>
            </div>
          </div>

          {/* Suggested Actions */}
          <div className="bg-blue-950/30 border border-blue-500/20 rounded p-3">
            <h4 className="text-blue-300 text-sm font-medium mb-2">What you can try:</h4>
            <ul className="text-blue-300/80 text-xs space-y-1">
              {getSuggestedActions().slice(0, 3).map((action, index) => (
                <li key={index} className="flex items-start gap-1">
                  <span className="text-blue-400 mt-0.5">•</span>
                  <span>{action}</span>
                </li>
              ))}
            </ul>
          </div>

          {/* Technical Details (collapsed by default) */}
          <details className="bg-gray-900 rounded p-3">
            <summary className="cursor-pointer text-gray-300 hover:text-white text-sm flex items-center gap-2">
              <span>🔧</span> Technical Details
            </summary>
            <div className="mt-2 space-y-2">
              <div className="text-xs">
                <span className="text-gray-400">Component:</span>
                <p className="text-gray-500 font-mono bg-gray-950 p-1 rounded mt-1">
                  {getComponentName()}
                </p>
              </div>
              <div className="text-xs">
                <span className="text-gray-400">Error:</span>
                <pre className="text-gray-500 font-mono bg-gray-950 p-1 rounded mt-1 text-xs overflow-auto">
                  {error.name}: {error.message}
                </pre>
              </div>
              {errorInfo && (
                <div className="text-xs">
                  <span className="text-gray-400">Component Stack:</span>
                  <pre className="text-gray-500 font-mono bg-gray-950 p-1 rounded mt-1 text-xs overflow-auto max-h-20">
                    {errorInfo.componentStack}
                  </pre>
                </div>
              )}
            </div>
          </details>
        </div>

        {/* Action Buttons */}
        <div className="flex flex-wrap gap-2">
          <button
            onClick={onRetry}
            disabled={isRecovering || retryCount >= 3}
            className="flex items-center gap-1 px-3 py-1.5 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <RefreshCw className={`h-3 w-3 ${isRecovering ? 'animate-spin' : ''}`} />
            {isRecovering ? 'Retrying...' : 'Retry'}
          </button>

          {isStateError && (
            <button
              onClick={handleResetState}
              className="flex items-center gap-1 px-3 py-1.5 bg-purple-600 text-white text-sm rounded hover:bg-purple-700 transition-colors"
            >
              <Settings className="h-3 w-3" />
              Reset State
            </button>
          )}

          <button
            onClick={onReset}
            className="flex items-center gap-1 px-3 py-1.5 bg-gray-600 text-white text-sm rounded hover:bg-gray-700 transition-colors"
          >
            <RefreshCw className="h-3 w-3" />
            Reset
          </button>

          <button
            onClick={() => window.location.reload()}
            className="flex items-center gap-1 px-3 py-1.5 bg-green-600 text-white text-sm rounded hover:bg-green-700 transition-colors"
          >
            <RefreshCw className="h-3 w-3" />
            Reload
          </button>
        </div>

        {/* Help Text */}
        <div className="mt-4 p-2 bg-blue-900/20 border border-blue-600/30 rounded text-xs">
          <p className="text-blue-300">
            💡 This component error doesn't affect the rest of the application. 
            Other features should continue working normally.
          </p>
        </div>
      </div>
    </div>
  );
}

export interface UIErrorBoundaryProps {
  children: React.ReactNode;
  componentName?: string;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
}

export function UIErrorBoundary({
  children,
  componentName,
  onError
}: UIErrorBoundaryProps) {
  const handleError = (error: Error, errorInfo: React.ErrorInfo) => {
    // Log UI-specific error details
    console.group(`🎨 UI Component Error in ${componentName || 'UI component'}`);
    console.error('Component:', componentName);
    console.error('Error:', error);
    console.error('Component Stack:', errorInfo.componentStack);
    
    // Extract useful debugging info
    const debugInfo = {
      componentName,
      errorType: error.name,
      errorMessage: error.message,
      reactVersion: React.version,
      timestamp: new Date().toISOString(),
      userAgent: navigator.userAgent
    };
    
    console.error('Debug Info:', debugInfo);
    console.groupEnd();

    if (onError) {
      onError(error, errorInfo);
    }
  };

  return (
    <ErrorBoundary
      onError={handleError}
      componentName={`UI ${componentName || 'Component'}`}
      enableReporting={false} // UI errors are usually not critical
      enableAutoRecovery={true}
      maxRetryAttempts={3}
      fallback={UIErrorFallback}
    >
      {children}
    </ErrorBoundary>
  );
}

export default UIErrorBoundary;