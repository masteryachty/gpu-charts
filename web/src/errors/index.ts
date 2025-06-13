/**
 * Error Handling System Exports
 * 
 * Centralized exports for the comprehensive error handling system
 */

// Core types and utilities
export * from './ErrorTypes';
export * from './ErrorHandler';

// React integration
export { useErrorHandler } from '../hooks/useErrorHandler';
export { ErrorBoundary, withErrorBoundary, useErrorBoundaryReset } from '../components/error/ErrorBoundary';
export { default as ErrorNotificationCenter, ErrorNotificationToggle } from '../components/error/ErrorNotificationCenter';

// Convenience exports for global error handling
export {
  getGlobalErrorHandler,
  setGlobalErrorHandler,
  handleError,
  handleWasmError,
  handleDataError,
  handleStoreError,
  handleNetworkError,
  handlePerformanceError,
  handleValidationError
} from './ErrorHandler';