import { useCallback, useEffect, useRef, useState } from 'react';
import { ErrorHandler, getGlobalErrorHandler } from '../errors/ErrorHandler';
import type { AppError, ErrorSummary } from '../errors/ErrorTypes';

/**
 * React Hook for Error Handling
 * 
 * Provides React components with comprehensive error handling capabilities,
 * including error reporting, recovery management, and user notifications.
 */

export interface UseErrorHandlerOptions {
  /** Use local error handler instance instead of global */
  useLocalHandler?: boolean;
  
  /** Enable automatic error boundary integration */
  enableErrorBoundary?: boolean;
  
  /** Categories of errors to listen for */
  subscribeToCategories?: string[];
  
  /** Severity levels to listen for */
  subscribeToSeverities?: string[];
  
  /** Maximum number of recent errors to track */
  maxRecentErrors?: number;
  
  /** Callback for when errors occur */
  onError?: (error: AppError) => void;
  
  /** Callback for successful error recovery */
  onRecovery?: (errorCode: string) => void;
}

export interface ErrorState {
  /** Current error if any */
  currentError: AppError | null;
  
  /** Recent errors */
  recentErrors: AppError[];
  
  /** Error summary for current session */
  errorSummary: ErrorSummary;
  
  /** Whether error handler is actively listening */
  isListening: boolean;
  
  /** Number of pending user notifications */
  pendingNotifications: number;
}

export interface ErrorHandlerAPI {
  /** Report an error */
  reportError: (error: AppError) => Promise<void>;
  
  /** Report a WASM error */
  reportWasmError: (code: string, message: string, context?: any) => Promise<void>;
  
  /** Report a data error */
  reportDataError: (code: string, message: string, context?: any) => Promise<void>;
  
  /** Report a store error */
  reportStoreError: (code: string, message: string, operation: any, context?: any) => Promise<void>;
  
  /** Report a network error */
  reportNetworkError: (code: string, message: string, context?: any) => Promise<void>;
  
  /** Report a performance error */
  reportPerformanceError: (code: string, message: string, metric: any, threshold: number, actual: number, context?: any) => Promise<void>;
  
  /** Report a validation error */
  reportValidationError: (code: string, message: string, field: string, value: any, constraint: string, context?: any) => Promise<void>;
  
  /** Clear current error */
  clearCurrentError: () => void;
  
  /** Clear all recent errors */
  clearRecentErrors: () => void;
  
  /** Get pending notifications and clear queue */
  getNotifications: () => AppError[];
  
  /** Register error recovery strategy */
  registerRecoveryStrategy: (strategy: any) => void;
  
  /** Start/stop listening for errors */
  setListening: (listening: boolean) => void;
  
  /** Manually trigger error summary refresh */
  refreshSummary: () => void;
}

export function useErrorHandler(
  options: UseErrorHandlerOptions = {}
): [ErrorState, ErrorHandlerAPI] {
  const {
    useLocalHandler = false,
    enableErrorBoundary = true,
    subscribeToCategories = [],
    subscribeToSeverities = [],
    maxRecentErrors = 50,
    onError,
    onRecovery
  } = options;
  
  // Error handler instance
  const errorHandlerRef = useRef<ErrorHandler | null>(null);
  const subscriptionIdsRef = useRef<string[]>([]);
  const mountedRef = useRef(true);
  
  // State management
  const [errorState, setErrorState] = useState<ErrorState>({
    currentError: null,
    recentErrors: [],
    errorSummary: {
      total: 0,
      bySeverity: { low: 0, medium: 0, high: 0, critical: 0 },
      byCategory: { wasm: 0, data: 0, store: 0, network: 0, performance: 0, validation: 0 },
      mostSevere: null,
      mostRecent: null,
      recoverableCount: 0
    },
    isListening: false,
    pendingNotifications: 0
  });
  
  // Initialize error handler
  useEffect(() => {
    if (useLocalHandler) {
      errorHandlerRef.current = new ErrorHandler();
    } else {
      errorHandlerRef.current = getGlobalErrorHandler();
    }
    
    // Add stack trace to see where this hook is being called from
    const stack = new Error().stack;
    console.log('[useErrorHandler] Error handler initialized');
    console.log('[useErrorHandler] Called from:', stack?.split('\n').slice(1, 6).join('\n'));
    
    return () => {
      if (useLocalHandler && errorHandlerRef.current) {
        errorHandlerRef.current.destroy();
      }
    };
  }, [useLocalHandler]);

  // Refresh summary function (defined early to avoid dependency issues)
  const refreshSummary = useCallback(() => {
    const handler = errorHandlerRef.current;
    if (!handler || !mountedRef.current) return;
    
    try {
      const summary = handler.getErrorSummary();
      
      // Only update if the summary actually changed
      setErrorState(prev => {
        const currentSummary = prev.errorSummary;
        const hasChanged = 
          currentSummary.total !== summary.total ||
          JSON.stringify(currentSummary.bySeverity) !== JSON.stringify(summary.bySeverity) ||
          JSON.stringify(currentSummary.byCategory) !== JSON.stringify(summary.byCategory);
        
        if (!hasChanged) {
          return prev; // No change, return same state
        }
        
        return {
          ...prev,
          errorSummary: summary
        };
      });
    } catch (error) {
      console.error('[useErrorHandler] Error refreshing summary:', error);
    }
  }, []);
  
  
  // Setup error subscriptions - using refs to avoid dependency issues
  useEffect(() => {
    const handler = errorHandlerRef.current;
    if (!handler) return;
    
    // Create stable callback that doesn't depend on changing refs
    const stableErrorCallback = (error: AppError) => {
      if (!mountedRef.current) return;
      
      console.log('[useErrorHandler] Received error event:', {
        code: error.code,
        category: error.category,
        severity: error.severity
      });
      
      setErrorState(prev => {
        const newRecentErrors = [error, ...prev.recentErrors.slice(0, maxRecentErrors - 1)];
        
        return {
          ...prev,
          currentError: error,
          recentErrors: newRecentErrors,
          pendingNotifications: prev.pendingNotifications + 1
        };
      });
      
      // Call user-provided error callback
      if (onError) {
        try {
          onError(error);
        } catch (callbackError) {
          console.error('[useErrorHandler] Error in onError callback:', callbackError);
        }
      }
    };
    
    // Clear existing subscriptions
    subscriptionIdsRef.current.forEach(id => handler.unsubscribe(id));
    subscriptionIdsRef.current = [];
    
    // Subscribe to all errors if no specific categories/severities
    if (subscribeToCategories.length === 0 && subscribeToSeverities.length === 0) {
      const id = handler.subscribe({
        callback: stableErrorCallback
      });
      subscriptionIdsRef.current.push(id);
    } else {
      // Subscribe to specific categories
      subscribeToCategories.forEach(category => {
        const id = handler.subscribe({
          category,
          callback: stableErrorCallback
        });
        subscriptionIdsRef.current.push(id);
      });
      
      // Subscribe to specific severities
      subscribeToSeverities.forEach(severity => {
        const id = handler.subscribe({
          severity,
          callback: stableErrorCallback
        });
        subscriptionIdsRef.current.push(id);
      });
    }
    
    setErrorState(prev => ({ ...prev, isListening: true }));
    
    return () => {
      subscriptionIdsRef.current.forEach(id => handler.unsubscribe(id));
      subscriptionIdsRef.current = [];
    };
  }, [maxRecentErrors, onError, subscribeToCategories, subscribeToSeverities]); // Include dependencies
  
  // Error reporting functions
  const reportError = useCallback(async (error: AppError): Promise<void> => {
    const handler = errorHandlerRef.current;
    if (handler) {
      await handler.handleError(error);
    }
  }, []);
  
  const reportWasmError = useCallback(async (code: string, message: string, context?: any): Promise<void> => {
    const handler = errorHandlerRef.current;
    if (handler) {
      await handler.handleWasmError(code, message, context);
    }
  }, []);
  
  const reportDataError = useCallback(async (code: string, message: string, context?: any): Promise<void> => {
    const handler = errorHandlerRef.current;
    if (handler) {
      await handler.handleDataError(code, message, context);
    }
  }, []);
  
  const reportStoreError = useCallback(async (code: string, message: string, operation: any, context?: any): Promise<void> => {
    const handler = errorHandlerRef.current;
    if (handler) {
      await handler.handleStoreError(code, message, operation, context);
    }
  }, []);
  
  const reportNetworkError = useCallback(async (code: string, message: string, context?: any): Promise<void> => {
    const handler = errorHandlerRef.current;
    if (handler) {
      await handler.handleNetworkError(code, message, context);
    }
  }, []);
  
  const reportPerformanceError = useCallback(async (code: string, message: string, metric: any, threshold: number, actual: number, context?: any): Promise<void> => {
    const handler = errorHandlerRef.current;
    if (handler) {
      await handler.handlePerformanceError(code, message, metric, threshold, actual, context);
    }
  }, []);
  
  const reportValidationError = useCallback(async (code: string, message: string, field: string, value: any, constraint: string, context?: any): Promise<void> => {
    const handler = errorHandlerRef.current;
    if (handler) {
      await handler.handleValidationError(code, message, field, value, constraint, context);
    }
  }, []);
  
  // State management functions
  const clearCurrentError = useCallback(() => {
    setErrorState(prev => ({ ...prev, currentError: null }));
  }, []);
  
  const clearRecentErrors = useCallback(() => {
    setErrorState(prev => ({
      ...prev,
      recentErrors: [],
      currentError: null,
      pendingNotifications: 0
    }));
  }, []);
  
  const getNotifications = useCallback((): AppError[] => {
    const handler = errorHandlerRef.current;
    if (!handler) return [];
    
    const notifications = handler.getNotificationQueue();
    
    setErrorState(prev => ({
      ...prev,
      pendingNotifications: 0
    }));
    
    return notifications;
  }, []);
  
  const registerRecoveryStrategy = useCallback((strategy: any) => {
    const handler = errorHandlerRef.current;
    if (handler) {
      handler.registerRecoveryStrategy(strategy);
    }
  }, []);
  
  const setListening = useCallback((listening: boolean) => {
    setErrorState(prev => ({ ...prev, isListening: listening }));
  }, []);
  
  // Refresh summary periodically - using ref to avoid dependency issues
  useEffect(() => {
    const refreshSummaryRef = () => {
      const handler = errorHandlerRef.current;
      if (!handler || !mountedRef.current) return;
      
      const summary = handler.getErrorSummary();
      
      setErrorState(prev => ({
        ...prev,
        errorSummary: summary
      }));
    };
    
    const interval = setInterval(refreshSummaryRef, 10000); // Every 10 seconds, less frequent
    return () => clearInterval(interval);
  }, []); // Empty dependency array to prevent recreation
  
  // Error boundary integration
  useEffect(() => {
    if (!enableErrorBoundary) return;
    
    const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
      console.error('[useErrorHandler] Unhandled promise rejection:', event.reason);
      
      reportError({
        category: 'network' as any,
        code: 'UNHANDLED_PROMISE_REJECTION',
        message: `Unhandled promise rejection: ${event.reason}`,
        timestamp: Date.now(),
        severity: 'high',
        context: {
          reason: event.reason,
          stack: event.reason?.stack
        }
      });
    };
    
    const handleError = (event: ErrorEvent) => {
      console.error('[useErrorHandler] Unhandled error:', event.error);
      
      reportError({
        category: 'validation' as any,
        code: 'UNHANDLED_ERROR',
        message: `Unhandled error: ${event.message}`,
        timestamp: Date.now(),
        severity: 'high',
        context: {
          filename: event.filename,
          lineno: event.lineno,
          colno: event.colno,
          error: event.error,
          stack: event.error?.stack
        }
      });
    };
    
    window.addEventListener('unhandledrejection', handleUnhandledRejection);
    window.addEventListener('error', handleError);
    
    return () => {
      window.removeEventListener('unhandledrejection', handleUnhandledRejection);
      window.removeEventListener('error', handleError);
    };
  }, [enableErrorBoundary, reportError]);
  
  // Cleanup on unmount
  useEffect(() => {
    mountedRef.current = true;
    
    return () => {
      mountedRef.current = false;
    };
  }, []);
  
  // API object
  const api: ErrorHandlerAPI = {
    reportError,
    reportWasmError,
    reportDataError,
    reportStoreError,
    reportNetworkError,
    reportPerformanceError,
    reportValidationError,
    clearCurrentError,
    clearRecentErrors,
    getNotifications,
    registerRecoveryStrategy,
    setListening,
    refreshSummary
  };
  
  return [errorState, api];
}