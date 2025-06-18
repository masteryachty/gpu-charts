import React, { Component, ReactNode } from 'react';
import { ErrorFactory, ERROR_CODES } from '../../errors/ErrorTypes';
// import { getGlobalErrorHandler } from '../../errors/ErrorHandler'; // Disabled temporarily

/**
 * Enhanced Error Boundary Component
 * 
 * Catches React rendering errors and integrates with the comprehensive
 * error handling system to provide recovery options and user feedback.
 */

export interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
  errorId: string | null;
  retryCount: number;
  isRecovering: boolean;
}

export interface ErrorBoundaryProps {
  children: ReactNode;
  
  /** Custom fallback component */
  fallback?: React.ComponentType<ErrorFallbackProps>;
  
  /** Enable automatic recovery attempts */
  enableAutoRecovery?: boolean;
  
  /** Maximum number of recovery attempts */
  maxRetryAttempts?: number;
  
  /** Callback when error occurs */
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
  
  /** Callback when error is cleared */
  onReset?: () => void;
  
  /** Component name for error context */
  componentName?: string;
  
  /** Enable detailed error reporting */
  enableReporting?: boolean;
}

export interface ErrorFallbackProps {
  error: Error;
  errorInfo: React.ErrorInfo;
  retryCount: number;
  isRecovering: boolean;
  onRetry: () => void;
  onReset: () => void;
  onReportError: () => void;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  private retryTimeouts: NodeJS.Timeout[] = [];
  
  constructor(props: ErrorBoundaryProps) {
    super(props);
    
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null,
      retryCount: 0,
      isRecovering: false
    };
  }
  
  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    // Update state to trigger error UI
    return {
      hasError: true,
      error,
      errorId: Math.random().toString(36).substr(2, 9)
    };
  }
  
  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('[ErrorBoundary] Caught React error:', error, errorInfo);
    
    this.setState({ errorInfo });
    
    // Report to error handler
    this.reportError(error, errorInfo);
    
    // Call user-provided error callback
    if (this.props.onError) {
      try {
        this.props.onError(error, errorInfo);
      } catch (callbackError) {
        console.error('[ErrorBoundary] Error in onError callback:', callbackError);
      }
    }
    
    // Attempt automatic recovery if enabled
    if (this.props.enableAutoRecovery && this.state.retryCount < (this.props.maxRetryAttempts || 3)) {
      this.scheduleRetry();
    }
  }
  
  private async reportError(error: Error, _errorInfo: React.ErrorInfo): Promise<void> {
    if (!this.props.enableReporting) return;
    
    // const errorHandler = getGlobalErrorHandler(); // Disabled temporarily
    
    // const appError = ErrorFactory.createWasmError(
    //   ERROR_CODES.WASM_CANVAS_ERROR,
    //   `React Error Boundary: ${error.message}`,
    //   {
    //     method: 'React.render',
    //     recoverable: true,
    //     wasmStack: error.stack
    //   }
    // );
    
    // await errorHandler.handleError(appError); // Disabled temporarily
    console.error('[ErrorBoundary] Error reported:', error.message);
  }
  
  private scheduleRetry = (): void => {
    const delay = Math.min(1000 * Math.pow(2, this.state.retryCount), 30000); // Exponential backoff, max 30s
    
    console.log(`[ErrorBoundary] Scheduling recovery attempt in ${delay}ms (attempt ${this.state.retryCount + 1})`);
    
    this.setState({ isRecovering: true });
    
    const timeout = setTimeout(() => {
      this.handleRetry();
    }, delay);
    
    this.retryTimeouts.push(timeout);
  };
  
  private handleRetry = (): void => {
    console.log(`[ErrorBoundary] Attempting recovery (attempt ${this.state.retryCount + 1})`);
    
    this.setState(prevState => ({
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null,
      retryCount: prevState.retryCount + 1,
      isRecovering: false
    }));
  };
  
  private handleManualRetry = (): void => {
    console.log('[ErrorBoundary] Manual retry requested');
    this.handleRetry();
  };
  
  private handleReset = (): void => {
    console.log('[ErrorBoundary] Reset requested');
    
    // Clear all retry timeouts
    this.retryTimeouts.forEach(timeout => clearTimeout(timeout));
    this.retryTimeouts = [];
    
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null,
      retryCount: 0,
      isRecovering: false
    });
    
    if (this.props.onReset) {
      this.props.onReset();
    }
  };
  
  private handleReportError = (): void => {
    if (this.state.error && this.state.errorInfo) {
      this.reportError(this.state.error, this.state.errorInfo);
    }
  };
  
  componentWillUnmount() {
    // Clear any pending retry timeouts
    this.retryTimeouts.forEach(timeout => clearTimeout(timeout));
  }
  
  render() {
    if (this.state.hasError && this.state.error && this.state.errorInfo) {
      // Use custom fallback component if provided
      if (this.props.fallback) {
        const FallbackComponent = this.props.fallback;
        return (
          <FallbackComponent
            error={this.state.error}
            errorInfo={this.state.errorInfo}
            retryCount={this.state.retryCount}
            isRecovering={this.state.isRecovering}
            onRetry={this.handleManualRetry}
            onReset={this.handleReset}
            onReportError={this.handleReportError}
          />
        );
      }
      
      // Default error UI
      return (
        <DefaultErrorFallback
          error={this.state.error}
          errorInfo={this.state.errorInfo}
          retryCount={this.state.retryCount}
          isRecovering={this.state.isRecovering}
          onRetry={this.handleManualRetry}
          onReset={this.handleReset}
          onReportError={this.handleReportError}
        />
      );
    }
    
    return this.props.children;
  }
}

/**
 * Default Error Fallback Component
 */
export function DefaultErrorFallback({
  error,
  errorInfo: _errorInfo,
  retryCount,
  isRecovering,
  onRetry,
  onReset,
  onReportError
}: ErrorFallbackProps) {
  const maxRetries = 3;
  const canRetry = retryCount < maxRetries;
  
  return (
    <div className="flex items-center justify-center min-h-screen bg-gray-900 p-6">
      <div className="max-w-2xl w-full bg-gray-800 border border-red-600 rounded-lg p-8">
        {/* Header */}
        <div className="flex items-center mb-6">
          <div className="w-12 h-12 bg-red-600 rounded-full flex items-center justify-center mr-4">
            <svg className="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
          </div>
          <div>
            <h1 className="text-2xl font-bold text-white mb-1">Something went wrong</h1>
            <p className="text-gray-400">The application encountered an unexpected error</p>
          </div>
        </div>
        
        {/* Error Details */}
        <div className="mb-6 space-y-4">
          <div>
            <h3 className="text-lg font-semibold text-white mb-2">Error Details</h3>
            <div className="bg-gray-900 border border-gray-600 rounded p-4">
              <p className="text-red-400 font-mono text-sm mb-2">{error.name}: {error.message}</p>
              {error.stack && (
                <details className="mt-2">
                  <summary className="text-gray-400 cursor-pointer hover:text-white">
                    View stack trace
                  </summary>
                  <pre className="mt-2 text-xs text-gray-500 overflow-x-auto whitespace-pre-wrap">
                    {error.stack}
                  </pre>
                </details>
              )}
            </div>
          </div>
          
          {/* Recovery Status */}
          <div>
            <h3 className="text-lg font-semibold text-white mb-2">Recovery Status</h3>
            <div className="bg-gray-900 border border-gray-600 rounded p-4">
              <div className="flex items-center justify-between mb-2">
                <span className="text-gray-400">Retry attempts:</span>
                <span className="text-white">{retryCount} / {maxRetries}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-gray-400">Status:</span>
                <span className={`text-sm px-2 py-1 rounded ${
                  isRecovering ? 'bg-yellow-600 text-yellow-100' :
                  canRetry ? 'bg-blue-600 text-blue-100' :
                  'bg-red-600 text-red-100'
                }`}>
                  {isRecovering ? 'Recovering...' : canRetry ? 'Ready to retry' : 'Max retries reached'}
                </span>
              </div>
            </div>
          </div>
        </div>
        
        {/* Action Buttons */}
        <div className="flex flex-wrap gap-3">
          {canRetry && (
            <button
              onClick={onRetry}
              disabled={isRecovering}
              className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {isRecovering ? 'Retrying...' : 'Try Again'}
            </button>
          )}
          
          <button
            onClick={onReset}
            className="px-6 py-3 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition-colors"
          >
            Reset Application
          </button>
          
          <button
            onClick={onReportError}
            className="px-6 py-3 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
          >
            Report Error
          </button>
          
          <button
            onClick={() => window.location.reload()}
            className="px-6 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors"
          >
            Reload Page
          </button>
        </div>
        
        {/* Help Text */}
        <div className="mt-6 p-4 bg-blue-900/30 border border-blue-600 rounded">
          <h4 className="text-blue-400 font-medium mb-2">What you can do:</h4>
          <ul className="text-blue-300 text-sm space-y-1">
            <li>• Try refreshing the page</li>
            <li>• Check your internet connection</li>
            <li>• Clear your browser cache</li>
            <li>• Contact support if the problem persists</li>
          </ul>
        </div>
      </div>
    </div>
  );
}

/**
 * Higher-order component for wrapping components with error boundary
 */
// eslint-disable-next-line react-refresh/only-export-components
export function withErrorBoundary<P extends object>(
  Component: React.ComponentType<P>,
  errorBoundaryProps?: Omit<ErrorBoundaryProps, 'children'>
) {
  const WrappedComponent = (props: P) => (
    <ErrorBoundary {...errorBoundaryProps}>
      <Component {...props} />
    </ErrorBoundary>
  );
  
  WrappedComponent.displayName = `withErrorBoundary(${Component.displayName || Component.name})`;
  
  return WrappedComponent;
}

/**
 * Hook for error boundary management in functional components
 */
// eslint-disable-next-line react-refresh/only-export-components
export function useErrorBoundaryReset() {
  const [key, setKey] = React.useState(0);
  
  const reset = React.useCallback(() => {
    setKey(prev => prev + 1);
  }, []);
  
  return { key, reset };
}