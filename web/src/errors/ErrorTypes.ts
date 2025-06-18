/**
 * Comprehensive Error Type System
 * 
 * Defines all error types across the React-Rust integration system,
 * including WASM errors, data fetching errors, and system-level errors.
 */

// Base error interface
export interface BaseError {
  code: string;
  message: string;
  timestamp: number;
  severity: 'low' | 'medium' | 'high' | 'critical';
  context?: Record<string, any>;
  stack?: string;
}

// WASM-specific errors
export interface WasmError extends BaseError {
  category: 'wasm';
  wasmMethod?: string;
  wasmStack?: string;
  recoverable: boolean;
}

// Data fetching errors
export interface DataError extends BaseError {
  category: 'data';
  endpoint?: string;
  requestId?: string;
  httpStatus?: number;
  retryable: boolean;
  retryAfter?: number;
}

// Store synchronization errors
export interface StoreError extends BaseError {
  category: 'store';
  storeState?: any;
  operation: 'sync' | 'update' | 'validation' | 'serialization';
  field?: string;
}

// Network connectivity errors
export interface NetworkError extends BaseError {
  category: 'network';
  url?: string;
  connectionType?: 'websocket' | 'http' | 'webgpu';
  timeoutMs?: number;
}

// Performance-related errors
export interface PerformanceError extends BaseError {
  category: 'performance';
  metric: 'memory' | 'fps' | 'latency' | 'cpu';
  threshold: number;
  actual: number;
  trend?: 'increasing' | 'stable' | 'decreasing';
}

// User input validation errors
export interface ValidationError extends BaseError {
  category: 'validation';
  field: string;
  value: any;
  constraint: string;
  suggestions?: string[];
}

// Union type for all errors
export type AppError = WasmError | DataError | StoreError | NetworkError | PerformanceError | ValidationError;

// Error severity levels
export enum ErrorSeverity {
  LOW = 'low',           // Minor issues, app continues normally
  MEDIUM = 'medium',     // Some functionality affected
  HIGH = 'high',         // Major functionality impaired
  CRITICAL = 'critical'  // App unusable, immediate action required
}

// Error categories
export enum ErrorCategory {
  WASM = 'wasm',
  DATA = 'data',
  STORE = 'store',
  NETWORK = 'network',
  PERFORMANCE = 'performance',
  VALIDATION = 'validation'
}

// Pre-defined error codes
export const ERROR_CODES = {
  // WASM errors
  WASM_INIT_FAILED: 'WASM_INIT_FAILED',
  WASM_METHOD_FAILED: 'WASM_METHOD_FAILED',
  WASM_MEMORY_ERROR: 'WASM_MEMORY_ERROR',
  WASM_WEBGPU_ERROR: 'WASM_WEBGPU_ERROR',
  WASM_CANVAS_ERROR: 'WASM_CANVAS_ERROR',
  
  // Data errors
  DATA_FETCH_FAILED: 'DATA_FETCH_FAILED',
  DATA_PARSE_ERROR: 'DATA_PARSE_ERROR',
  DATA_CACHE_ERROR: 'DATA_CACHE_ERROR',
  DATA_TIMEOUT: 'DATA_TIMEOUT',
  DATA_INVALID_RESPONSE: 'DATA_INVALID_RESPONSE',
  
  // Store errors
  STORE_SYNC_FAILED: 'STORE_SYNC_FAILED',
  STORE_VALIDATION_ERROR: 'STORE_VALIDATION_ERROR',
  STORE_SERIALIZATION_ERROR: 'STORE_SERIALIZATION_ERROR',
  STORE_STATE_CORRUPT: 'STORE_STATE_CORRUPT',
  
  // Network errors
  NETWORK_OFFLINE: 'NETWORK_OFFLINE',
  NETWORK_TIMEOUT: 'NETWORK_TIMEOUT',
  NETWORK_SERVER_ERROR: 'NETWORK_SERVER_ERROR',
  NETWORK_WEBSOCKET_ERROR: 'NETWORK_WEBSOCKET_ERROR',
  
  // Performance errors
  PERFORMANCE_MEMORY_LEAK: 'PERFORMANCE_MEMORY_LEAK',
  PERFORMANCE_LOW_FPS: 'PERFORMANCE_LOW_FPS',
  PERFORMANCE_HIGH_LATENCY: 'PERFORMANCE_HIGH_LATENCY',
  PERFORMANCE_CPU_OVERLOAD: 'PERFORMANCE_CPU_OVERLOAD',
  
  // Validation errors
  VALIDATION_INVALID_SYMBOL: 'VALIDATION_INVALID_SYMBOL',
  VALIDATION_INVALID_TIMEFRAME: 'VALIDATION_INVALID_TIMEFRAME',
  VALIDATION_INVALID_TIME_RANGE: 'VALIDATION_INVALID_TIME_RANGE',
  VALIDATION_MISSING_FIELD: 'VALIDATION_MISSING_FIELD'
} as const;

// Error factory functions
export class ErrorFactory {
  static createWasmError(
    code: string,
    message: string,
    context?: {
      method?: string;
      wasmStack?: string;
      recoverable?: boolean;
    }
  ): WasmError {
    return {
      category: 'wasm',
      code,
      message,
      timestamp: Date.now(),
      severity: 'high',
      wasmMethod: context?.method,
      wasmStack: context?.wasmStack,
      recoverable: context?.recoverable ?? false,
      context
    };
  }
  
  static createDataError(
    code: string,
    message: string,
    context?: {
      endpoint?: string;
      requestId?: string;
      httpStatus?: number;
      retryable?: boolean;
      retryAfter?: number;
    }
  ): DataError {
    return {
      category: 'data',
      code,
      message,
      timestamp: Date.now(),
      severity: 'medium',
      endpoint: context?.endpoint,
      requestId: context?.requestId,
      httpStatus: context?.httpStatus,
      retryable: context?.retryable ?? true,
      retryAfter: context?.retryAfter,
      context
    };
  }
  
  static createStoreError(
    code: string,
    message: string,
    operation: StoreError['operation'],
    context?: {
      storeState?: any;
      field?: string;
    }
  ): StoreError {
    return {
      category: 'store',
      code,
      message,
      timestamp: Date.now(),
      severity: 'medium',
      operation,
      storeState: context?.storeState,
      field: context?.field,
      context
    };
  }
  
  static createNetworkError(
    code: string,
    message: string,
    context?: {
      url?: string;
      connectionType?: NetworkError['connectionType'];
      timeoutMs?: number;
    }
  ): NetworkError {
    return {
      category: 'network',
      code,
      message,
      timestamp: Date.now(),
      severity: 'high',
      url: context?.url,
      connectionType: context?.connectionType,
      timeoutMs: context?.timeoutMs,
      context
    };
  }
  
  static createPerformanceError(
    code: string,
    message: string,
    metric: PerformanceError['metric'],
    threshold: number,
    actual: number,
    context?: {
      trend?: PerformanceError['trend'];
    }
  ): PerformanceError {
    return {
      category: 'performance',
      code,
      message,
      timestamp: Date.now(),
      severity: actual > threshold * 2 ? 'critical' : 'medium',
      metric,
      threshold,
      actual,
      trend: context?.trend,
      context
    };
  }
  
  static createValidationError(
    code: string,
    message: string,
    field: string,
    value: any,
    constraint: string,
    context?: {
      suggestions?: string[];
    }
  ): ValidationError {
    return {
      category: 'validation',
      code,
      message,
      timestamp: Date.now(),
      severity: 'low',
      field,
      value,
      constraint,
      suggestions: context?.suggestions,
      context
    };
  }
}

// Error serialization utilities
export function serializeError(error: AppError): string {
  return JSON.stringify(error, (key, value) => {
    // Handle stack traces and circular references
    if (key === 'stack' && typeof value === 'string') {
      return value.split('\n').slice(0, 10).join('\n'); // Limit stack trace
    }
    return value;
  });
}

export function deserializeError(json: string): AppError {
  return JSON.parse(json);
}

// Error comparison utilities
export function isSameError(error1: AppError, error2: AppError): boolean {
  return (
    error1.code === error2.code &&
    error1.category === error2.category &&
    error1.message === error2.message
  );
}

export function isRecoverableError(error: AppError): boolean {
  switch (error.category) {
    case 'wasm':
      return (error as WasmError).recoverable;
    case 'data':
      return (error as DataError).retryable;
    case 'network':
      return true; // Network errors are generally recoverable
    case 'store':
      return error.code !== ERROR_CODES.STORE_STATE_CORRUPT;
    case 'performance':
      return true; // Performance issues can be mitigated
    case 'validation':
      return true; // Validation errors can be corrected
    default:
      return false;
  }
}

export function getErrorSeverityLevel(error: AppError): number {
  switch (error.severity) {
    case 'low': return 1;
    case 'medium': return 2;
    case 'high': return 3;
    case 'critical': return 4;
    default: return 0;
  }
}

// Error aggregation for multiple errors
export interface ErrorSummary {
  total: number;
  bySeverity: Record<ErrorSeverity, number>;
  byCategory: Record<ErrorCategory, number>;
  mostSevere: AppError | null;
  mostRecent: AppError | null;
  recoverableCount: number;
}

export function summarizeErrors(errors: AppError[]): ErrorSummary {
  const summary: ErrorSummary = {
    total: errors.length,
    bySeverity: {
      [ErrorSeverity.LOW]: 0,
      [ErrorSeverity.MEDIUM]: 0,
      [ErrorSeverity.HIGH]: 0,
      [ErrorSeverity.CRITICAL]: 0
    },
    byCategory: {
      [ErrorCategory.WASM]: 0,
      [ErrorCategory.DATA]: 0,
      [ErrorCategory.STORE]: 0,
      [ErrorCategory.NETWORK]: 0,
      [ErrorCategory.PERFORMANCE]: 0,
      [ErrorCategory.VALIDATION]: 0
    },
    mostSevere: null,
    mostRecent: null,
    recoverableCount: 0
  };
  
  let maxSeverityLevel = 0;
  let mostRecentTimestamp = 0;
  
  for (const error of errors) {
    // Count by severity
    summary.bySeverity[error.severity]++;
    
    // Count by category
    summary.byCategory[error.category]++;
    
    // Track most severe error
    const severityLevel = getErrorSeverityLevel(error);
    if (severityLevel > maxSeverityLevel) {
      maxSeverityLevel = severityLevel;
      summary.mostSevere = error;
    }
    
    // Track most recent error
    if (error.timestamp > mostRecentTimestamp) {
      mostRecentTimestamp = error.timestamp;
      summary.mostRecent = error;
    }
    
    // Count recoverable errors
    if (isRecoverableError(error)) {
      summary.recoverableCount++;
    }
  }
  
  return summary;
}