/**
 * Error Handling System Exports
 * 
 * Centralized exports for the comprehensive error handling system
 */

// Core types and utilities
export * from './ErrorTypes';
// export * from './ErrorHandler'; // Disabled temporarily

// React integration
// export { useErrorHandler } from '../hooks/useErrorHandler'; // Disabled temporarily
export { ErrorBoundary, withErrorBoundary, useErrorBoundaryReset } from '../components/error/ErrorBoundary';
export { default as ErrorNotificationCenter, ErrorNotificationToggle } from '../components/error/ErrorNotificationCenter';

// Convenience exports for global error handling - Disabled temporarily
// export {
//   getGlobalErrorHandler,
//   setGlobalErrorHandler,
//   handleError,
//   handleWasmError,
//   handleDataError,
//   handleStoreError,
//   handleNetworkError,
//   handlePerformanceError,
//   handleValidationError
// } from './ErrorHandler';