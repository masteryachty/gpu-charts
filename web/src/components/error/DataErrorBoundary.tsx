import React from 'react';
import { ErrorBoundary } from './ErrorBoundary';
import { WifiOff, RefreshCw, AlertTriangle, Database, Clock } from 'lucide-react';
import type { ErrorFallbackProps } from './ErrorBoundary';

/**
 * Specialized error boundary for data loading and API operations
 * Provides specific recovery options for network and data-related failures
 */

function DataErrorFallback({
  error,
  errorInfo,
  retryCount,
  isRecovering,
  onRetry,
  onReset,
  onReportError
}: ErrorFallbackProps) {
  const isNetworkError = error.message.toLowerCase().includes('network') || 
                        error.message.toLowerCase().includes('fetch') ||
                        error.message.toLowerCase().includes('timeout') ||
                        error.message.toLowerCase().includes('cors');
  
  const isApiError = error.message.toLowerCase().includes('api') ||
                     error.message.toLowerCase().includes('server') ||
                     error.message.toLowerCase().includes('endpoint');

  const isDataError = error.message.toLowerCase().includes('parse') ||
                      error.message.toLowerCase().includes('invalid') ||
                      error.message.toLowerCase().includes('format');

  const getErrorType = () => {
    if (isNetworkError) return 'Network Connection';
    if (isApiError) return 'API Server';
    if (isDataError) return 'Data Parsing';
    return 'Data Loading';
  };

  const getErrorIcon = () => {
    if (isNetworkError) return <WifiOff className="h-8 w-8 text-red-400" />;
    if (isApiError) return <Database className="h-8 w-8 text-orange-400" />;
    if (isDataError) return <AlertTriangle className="h-8 w-8 text-yellow-400" />;
    return <Clock className="h-8 w-8 text-red-400" />;
  };

  const getErrorDescription = () => {
    if (isNetworkError) {
      return 'Unable to connect to the data server. Please check your internet connection.';
    }
    if (isApiError) {
      return 'The data API is currently unavailable or returned an error.';
    }
    if (isDataError) {
      return 'The received data could not be processed. It may be corrupted or in an unexpected format.';
    }
    return 'An error occurred while loading market data.';
  };

  const getSuggestedActions = () => {
    const actions = [];
    
    if (isNetworkError) {
      actions.push('Check your internet connection');
      actions.push('Verify you can access other websites');
      actions.push('Try disabling VPN or proxy if enabled');
      actions.push('Check if your firewall is blocking the connection');
    } else if (isApiError) {
      actions.push('The server may be temporarily unavailable');
      actions.push('Try again in a few moments');
      actions.push('Check the API status at api.rednax.io');
      actions.push('Contact support if the problem persists');
    } else if (isDataError) {
      actions.push('Try refreshing to get fresh data');
      actions.push('Select a different time range or symbol');
      actions.push('Clear browser cache and reload');
    } else {
      actions.push('Refresh the page to retry data loading');
      actions.push('Try selecting a different symbol or time range');
      actions.push('Check your network connection');
    }
    
    return actions;
  };

  const getRetryDelayMessage = () => {
    if (retryCount === 0) return '';
    const delay = Math.min(1000 * Math.pow(2, retryCount - 1), 30000);
    return `(Next retry in ${delay / 1000}s)`;
  };

  const handleClearCache = () => {
    // Clear relevant caches
    if ('caches' in window) {
      caches.keys().then(names => {
        names.forEach(name => {
          caches.delete(name);
        });
      });
    }
    
    // Clear localStorage data related to charts
    Object.keys(localStorage).forEach(key => {
      if (key.includes('chart') || key.includes('market') || key.includes('symbol')) {
        localStorage.removeItem(key);
      }
    });
    
    onRetry();
  };

  const handleRetryWithDifferentEndpoint = () => {
    // Switch to fallback API endpoint if available
    const currentUrl = localStorage.getItem('api_base_url') || process.env.REACT_APP_API_BASE_URL;
    if (currentUrl?.includes('api.rednax.io')) {
      localStorage.setItem('api_base_url', 'https://localhost:8443');
    } else {
      localStorage.setItem('api_base_url', 'https://api.rednax.io');
    }
    onRetry();
  };

  return (
    <div className="flex-1 flex items-center justify-center bg-gray-900 p-6">
      <div className="max-w-2xl w-full bg-gray-800 border border-orange-500/30 rounded-lg p-8">
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
          <div className="bg-orange-950/30 border border-orange-500/20 rounded p-4">
            <div className="flex items-start gap-3">
              <AlertTriangle className="h-5 w-5 text-orange-400 mt-0.5 flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-orange-300 font-medium mb-2">
                  {error.message}
                </p>
                <div className="text-orange-400/80 text-sm space-y-1">
                  <p>Retry attempts: {retryCount} / 3 {getRetryDelayMessage()}</p>
                  {isRecovering && (
                    <p className="flex items-center gap-2">
                      <RefreshCw className="h-4 w-4 animate-spin" />
                      Attempting to reconnect...
                    </p>
                  )}
                </div>
              </div>
            </div>
          </div>

          {/* Connection Status */}
          <div className="bg-blue-950/30 border border-blue-500/20 rounded p-4">
            <h4 className="text-blue-300 font-medium mb-3 flex items-center gap-2">
              <Database className="h-4 w-4" />
              Connection Status
            </h4>
            <div className="text-blue-300/80 text-sm space-y-2">
              <div className="flex justify-between">
                <span>Navigator Online:</span>
                <span className={navigator.onLine ? 'text-green-400' : 'text-red-400'}>
                  {navigator.onLine ? '✓ Connected' : '✗ Offline'}
                </span>
              </div>
              <div className="flex justify-between">
                <span>Current API:</span>
                <span className="text-blue-400 font-mono text-xs">
                  {process.env.REACT_APP_API_BASE_URL || 'api.rednax.io'}
                </span>
              </div>
              <div className="flex justify-between">
                <span>Error Type:</span>
                <span className="text-yellow-400">{getErrorType()}</span>
              </div>
            </div>
          </div>

          {/* Suggested Actions */}
          <div className="bg-green-950/30 border border-green-500/20 rounded p-4">
            <h4 className="text-green-300 font-medium mb-3">What you can try:</h4>
            <ul className="text-green-300/80 text-sm space-y-2">
              {getSuggestedActions().map((action, index) => (
                <li key={index} className="flex items-start gap-2">
                  <span className="text-green-400 mt-1">•</span>
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
                <span className="text-gray-400">Error:</span>
                <pre className="mt-1 text-xs text-gray-500 overflow-auto whitespace-pre-wrap bg-gray-950 p-2 rounded">
                  {error.name}: {error.message}
                </pre>
              </div>
              {error.stack && (
                <div className="text-sm">
                  <span className="text-gray-400">Stack Trace:</span>
                  <pre className="mt-1 text-xs text-gray-500 overflow-auto whitespace-pre-wrap bg-gray-950 p-2 rounded">
                    {error.stack}
                  </pre>
                </div>
              )}
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
            <RefreshCw className={`h-4 w-4 ${isRecovering ? 'animate-spin' : ''}`} />
            {isRecovering ? 'Retrying...' : 'Retry'}
          </button>

          <button
            onClick={handleClearCache}
            className="flex items-center gap-2 px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 transition-colors"
          >
            <Database className="h-4 w-4" />
            Clear Cache & Retry
          </button>

          {isNetworkError && (
            <button
              onClick={handleRetryWithDifferentEndpoint}
              className="flex items-center gap-2 px-4 py-2 bg-yellow-600 text-white rounded hover:bg-yellow-700 transition-colors"
            >
              <WifiOff className="h-4 w-4" />
              Try Different Server
            </button>
          )}

          <button
            onClick={onReset}
            className="flex items-center gap-2 px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700 transition-colors"
          >
            <RefreshCw className="h-4 w-4" />
            Reset
          </button>

          <button
            onClick={() => window.location.reload()}
            className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors"
          >
            <RefreshCw className="h-4 w-4" />
            Reload Page
          </button>

          <button
            onClick={onReportError}
            className="flex items-center gap-2 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 transition-colors"
          >
            🐛 Report Issue
          </button>
        </div>
      </div>
    </div>
  );
}

export interface DataErrorBoundaryProps {
  children: React.ReactNode;
  dataSource?: string;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
}

export function DataErrorBoundary({
  children,
  dataSource,
  onError
}: DataErrorBoundaryProps) {
  const handleError = (error: Error, errorInfo: React.ErrorInfo) => {
    // Log data-specific error details
    console.group(`📊 Data Loading Error in ${dataSource || 'data component'}`);
    console.error('Data Source:', dataSource);
    console.error('Online Status:', navigator.onLine);
    console.error('API Base URL:', process.env.REACT_APP_API_BASE_URL);
    console.error('Error:', error);
    console.error('Component Stack:', errorInfo.componentStack);
    console.groupEnd();

    // Check network conditions
    const networkInfo = {
      online: navigator.onLine,
      connection: (navigator as any).connection,
      apiUrl: process.env.REACT_APP_API_BASE_URL,
      timestamp: new Date().toISOString()
    };

    console.info('Network Info:', networkInfo);

    if (onError) {
      onError(error, errorInfo);
    }
  };

  return (
    <ErrorBoundary
      onError={handleError}
      componentName={`Data Loader ${dataSource || ''}`}
      enableReporting={true}
      enableAutoRecovery={true}
      maxRetryAttempts={3}
      fallback={DataErrorFallback}
    >
      {children}
    </ErrorBoundary>
  );
}

export default DataErrorBoundary;