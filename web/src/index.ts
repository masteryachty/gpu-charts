/**
 * React Store → Rust Integration System
 * 
 * Complete index file exporting all public APIs for the comprehensive
 * React-Rust integration system.
 */

// Core types and utilities
export type * from './types';
export type * from './types/advanced-types';
export type * from './types/type-guards';

// Store management
export { useAppStore } from './store/useAppStore';

// React hooks
export { useWasmChart } from './hooks/useWasmChart';
export { useErrorHandler } from './hooks/useErrorHandler';
export { useAutonomousDataFetching } from './hooks/useAutonomousDataFetching';

// Services
export { DataFetchingService } from './services/DataFetchingService';

// Error handling system
export * from './errors';

// Performance monitoring
import { getGlobalPerformanceMonitor as getGlobalPerformanceMonitorImpl } from './performance/PerformanceMonitor';
export { PerformanceMonitor, getGlobalPerformanceMonitor } from './performance/PerformanceMonitor';

// React components
export { ErrorBoundary, withErrorBoundary, useErrorBoundaryReset } from './components/error/ErrorBoundary';
export { default as ErrorNotificationCenter, ErrorNotificationToggle } from './components/error/ErrorNotificationCenter';
export { default as DataFetchingMonitor } from './components/monitoring/DataFetchingMonitor';
export { default as WasmCanvas } from './components/chart/WasmCanvas';

// Re-export commonly used interfaces for convenience
export type {
  StoreState,
  ChartConfig,
  MarketData,
  AppError,
  PerformanceMetrics,
  DataFetchRequest,
  DataFetchResponse,
  WasmChartState,
  WasmChartAPI,
  ErrorState,
  ErrorHandlerAPI,
  DataFetchingState,
  DataFetchingAPI
} from './types';

// Version information
export const VERSION = '1.0.0';
export const BUILD_DATE = new Date().toISOString();

/**
 * Initialize the React Store → Rust Integration System
 * 
 * This function sets up the global configuration and initializes
 * the core services for the integration system.
 */
export function initializeIntegrationSystem(config?: {
  enableErrorReporting?: boolean;
  enablePerformanceMonitoring?: boolean;
  debugMode?: boolean;
}) {
  const {
    enableErrorReporting = true,
    enablePerformanceMonitoring = true,
    debugMode = false
  } = config || {};

  // Log initialization
  console.log(`[IntegrationSystem] Initializing React Store → Rust Integration v${VERSION}`);
  
  if (debugMode) {
    console.log('[IntegrationSystem] Debug mode enabled');
    (window as any).__INTEGRATION_DEBUG__ = true;
  }

  if (enableErrorReporting) {
    console.log('[IntegrationSystem] Error reporting enabled');
  }

  if (enablePerformanceMonitoring) {
    console.log('[IntegrationSystem] Performance monitoring enabled');
    // Start global performance monitor
    const monitor = getGlobalPerformanceMonitorImpl();
    monitor.startMonitoring(1000);
  }

  console.log('[IntegrationSystem] Initialization complete');
}