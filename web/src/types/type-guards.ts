/**
 * Comprehensive Type Guards and Runtime Validation
 * 
 * Advanced type guards for runtime type safety across the React-Rust integration system.
 */

import {
  SymbolId,
  Timestamp,
  Price,
  Volume,
  Percentage,
  Column,
  ColumnValues,
  ErrorSeverity,
  ErrorSeverityValues,
  ErrorCategory,
  ErrorCategoryValues
} from './advanced-types';

// Primitive type guards
export function isString(value: unknown): value is string {
  return typeof value === 'string';
}

export function isNumber(value: unknown): value is number {
  return typeof value === 'number' && !Number.isNaN(value);
}

export function isBoolean(value: unknown): value is boolean {
  return typeof value === 'boolean';
}

export function isArray<T>(value: unknown, itemGuard?: (item: unknown) => item is T): value is T[] {
  if (!Array.isArray(value)) return false;
  if (itemGuard) {
    return value.every(itemGuard);
  }
  return true;
}

export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function isNonEmptyString(value: unknown): value is string {
  return isString(value) && value.trim().length > 0;
}

export function isPositiveNumber(value: unknown): value is number {
  return isNumber(value) && value > 0;
}

export function isNonNegativeNumber(value: unknown): value is number {
  return isNumber(value) && value >= 0;
}

export function isInteger(value: unknown): value is number {
  return isNumber(value) && Number.isInteger(value);
}

export function isFiniteNumber(value: unknown): value is number {
  return isNumber(value) && Number.isFinite(value);
}

// Domain-specific type guards
export function isSymbolId(value: unknown): value is SymbolId {
  return isString(value) && /^[A-Z]{2,10}-[A-Z]{2,10}$/.test(value);
}

export function isTimestamp(value: unknown): value is Timestamp {
  return isInteger(value) && value > 0 && value <= Date.now() + 365 * 24 * 60 * 60 * 1000; // Max 1 year in future
}

export function isPrice(value: unknown): value is Price {
  return isFiniteNumber(value) && value >= 0;
}

export function isVolume(value: unknown): value is Volume {
  return isFiniteNumber(value) && value >= 0;
}

export function isPercentage(value: unknown): value is Percentage {
  return isFiniteNumber(value) && value >= -100 && value <= 1000; // Reasonable percentage range
}


export function isColumn(value: unknown): value is Column {
  return isString(value) && ColumnValues.includes(value as Column);
}

export function isErrorSeverity(value: unknown): value is ErrorSeverity {
  return isString(value) && ErrorSeverityValues.includes(value as ErrorSeverity);
}

export function isErrorCategory(value: unknown): value is ErrorCategory {
  return isString(value) && ErrorCategoryValues.includes(value as ErrorCategory);
}

// Complex object type guards
export interface ChartStateConfig {
  symbol: SymbolId;
  startTime: Timestamp;
  endTime: Timestamp;
}

export function isChartStateConfig(value: unknown): value is ChartStateConfig {
  if (!isObject(value)) return false;

  const config = value as any;

  return (
    isSymbolId(config.symbol) &&
    isTimestamp(config.startTime) &&
    isTimestamp(config.endTime) &&
    config.startTime < config.endTime
  );
}

export interface MarketData {
  symbol: SymbolId;
  price: Price;
  change: number;
  changePercent: Percentage;
  volume: Volume;
  timestamp: Timestamp;
}

export function isMarketData(value: unknown): value is MarketData {
  if (!isObject(value)) return false;

  const data = value as any;

  return (
    isSymbolId(data.symbol) &&
    isPrice(data.price) &&
    isFiniteNumber(data.change) &&
    isPercentage(data.changePercent) &&
    isVolume(data.volume) &&
    isTimestamp(data.timestamp)
  );
}

export interface StoreState {
  currentSymbol: SymbolId;
  ChartStateConfig: ChartStateConfig;
  marketData: Record<string, MarketData>;
  isConnected: boolean;
  user?: {
    id: string;
    name: string;
    email: string;
  };
}

export function isStoreState(value: unknown): value is StoreState {
  if (!isObject(value)) return false;

  const state = value as any;

  return (
    isSymbolId(state.currentSymbol) &&
    isChartStateConfig(state.ChartStateConfig) &&
    isObject(state.marketData) &&
    Object.values(state.marketData).every(isMarketData) &&
    isBoolean(state.isConnected) &&
    (state.user === undefined || isUserObject(state.user))
  );
}

function isUserObject(value: unknown): value is { id: string; name: string; email: string } {
  if (!isObject(value)) return false;

  const user = value as any;

  return (
    isNonEmptyString(user.id) &&
    isNonEmptyString(user.name) &&
    isNonEmptyString(user.email) &&
    /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(user.email)
  );
}

// Performance metrics type guards
export interface PerformanceMetrics {
  fps: number;
  frameTime: number;
  renderLatency: number;
  jsHeapUsed: number;
  jsHeapTotal: number;
  wasmMemoryUsed: number;
  totalMemoryUsage: number;
  networkLatency: number;
  cpuUsage: number;
  timestamp: Timestamp;
  systemHealth: 'excellent' | 'good' | 'fair' | 'poor' | 'critical';
}

export function isPerformanceMetrics(value: unknown): value is PerformanceMetrics {
  if (!isObject(value)) return false;

  const metrics = value as any;

  return (
    isNonNegativeNumber(metrics.fps) &&
    isNonNegativeNumber(metrics.frameTime) &&
    isNonNegativeNumber(metrics.renderLatency) &&
    isNonNegativeNumber(metrics.jsHeapUsed) &&
    isNonNegativeNumber(metrics.jsHeapTotal) &&
    isNonNegativeNumber(metrics.wasmMemoryUsed) &&
    isNonNegativeNumber(metrics.totalMemoryUsage) &&
    isNonNegativeNumber(metrics.networkLatency) &&
    isNonNegativeNumber(metrics.cpuUsage) &&
    metrics.cpuUsage <= 100 &&
    isTimestamp(metrics.timestamp) &&
    ['excellent', 'good', 'fair', 'poor', 'critical'].includes(metrics.systemHealth)
  );
}

// Error object type guards
export interface AppError {
  code: string;
  message: string;
  timestamp: Timestamp;
  severity: ErrorSeverity;
  category: ErrorCategory;
  context?: Record<string, unknown>;
}

export function isAppError(value: unknown): value is AppError {
  if (!isObject(value)) return false;

  const error = value as any;

  return (
    isNonEmptyString(error.code) &&
    isNonEmptyString(error.message) &&
    isTimestamp(error.timestamp) &&
    isErrorSeverity(error.severity) &&
    isErrorCategory(error.category) &&
    (error.context === undefined || isObject(error.context))
  );
}

// Data fetching type guards
export interface DataFetchRequest {
  symbol: SymbolId;
  startTime: Timestamp;
  endTime: Timestamp;
  columns: Column[];
  priority: 'low' | 'normal' | 'high' | 'critical';
  reason: 'user_action' | 'auto_sync' | 'prefetch' | 'real_time';
}

export function isDataFetchRequest(value: unknown): value is DataFetchRequest {
  if (!isObject(value)) return false;

  const request = value as any;

  return (
    isSymbolId(request.symbol) &&
    isTimestamp(request.startTime) &&
    isTimestamp(request.endTime) &&
    request.startTime < request.endTime &&
    isArray(request.columns, isColumn) &&
    request.columns.length > 0 &&
    ['low', 'normal', 'high', 'critical'].includes(request.priority) &&
    ['user_action', 'auto_sync', 'prefetch', 'real_time'].includes(request.reason)
  );
}

export interface DataFetchResponse {
  success: boolean;
  data?: ArrayBuffer;
  metadata?: {
    symbol: SymbolId;
    timeRange: [Timestamp, Timestamp];
    recordCount: number;
    fetchTime: number;
    cacheHit: boolean;
  };
  error?: string;
  retryAfter?: number;
}

export function isDataFetchResponse(value: unknown): value is DataFetchResponse {
  if (!isObject(value)) return false;

  const response = value as any;

  if (!isBoolean(response.success)) return false;

  if (response.success) {
    return (
      (response.data === undefined || response.data instanceof ArrayBuffer) &&
      (response.metadata === undefined || isDataFetchMetadata(response.metadata))
    );
  } else {
    return (
      isNonEmptyString(response.error) &&
      (response.retryAfter === undefined || isPositiveNumber(response.retryAfter))
    );
  }
}

function isDataFetchMetadata(value: unknown): value is DataFetchResponse['metadata'] {
  if (!isObject(value)) return false;

  const metadata = value as any;

  return (
    isSymbolId(metadata.symbol) &&
    isArray(metadata.timeRange) &&
    metadata.timeRange.length === 2 &&
    isTimestamp(metadata.timeRange[0]) &&
    isTimestamp(metadata.timeRange[1]) &&
    metadata.timeRange[0] < metadata.timeRange[1] &&
    isNonNegativeNumber(metadata.recordCount) &&
    isNonNegativeNumber(metadata.fetchTime) &&
    isBoolean(metadata.cacheHit)
  );
}

// Configuration type guards
export interface ConfigurationObject {
  chart: {
    maxDataPoints: number;
    refreshInterval: number;
  };
  performance: {
    fpsThreshold: number;
    memoryThreshold: number;
    enableOptimizations: boolean;
  };
  data: {
    cacheSize: number;
    retryAttempts: number;
    timeoutMs: number;
  };
  errors: {
    enableReporting: boolean;
    maxErrorHistory: number;
    autoRecovery: boolean;
  };
}

export function isConfigurationObject(value: unknown): value is ConfigurationObject {
  if (!isObject(value)) return false;

  const config = value as any;

  return (
    isChartStateConfigSection(config.chart) &&
    isPerformanceConfigSection(config.performance) &&
    isDataConfigSection(config.data) &&
    isErrorsConfigSection(config.errors)
  );
}

function isChartStateConfigSection(value: unknown): boolean {
  if (!isObject(value)) return false;

  const chart = value as any;

  return (
    isPositiveNumber(chart.maxDataPoints) &&
    isPositiveNumber(chart.refreshInterval)
  );
}

function isPerformanceConfigSection(value: unknown): boolean {
  if (!isObject(value)) return false;

  const perf = value as any;

  return (
    isPositiveNumber(perf.fpsThreshold) &&
    isPositiveNumber(perf.memoryThreshold) &&
    isBoolean(perf.enableOptimizations)
  );
}

function isDataConfigSection(value: unknown): boolean {
  if (!isObject(value)) return false;

  const data = value as any;

  return (
    isPositiveNumber(data.cacheSize) &&
    isPositiveNumber(data.retryAttempts) &&
    isPositiveNumber(data.timeoutMs)
  );
}

function isErrorsConfigSection(value: unknown): boolean {
  if (!isObject(value)) return false;

  const errors = value as any;

  return (
    isBoolean(errors.enableReporting) &&
    isPositiveNumber(errors.maxErrorHistory) &&
    isBoolean(errors.autoRecovery)
  );
}

// Composite validation functions
export function validateAndTransform<T>(
  value: unknown,
  guard: (value: unknown) => value is T,
  errorMessage: string
): T {
  if (guard(value)) {
    return value;
  }
  throw new Error(`Validation failed: ${errorMessage}. Received: ${JSON.stringify(value)}`);
}

export function validateArrayItems<T>(
  value: unknown,
  itemGuard: (item: unknown) => item is T,
  errorMessage: string
): T[] {
  if (!Array.isArray(value)) {
    throw new Error(`Expected array, received: ${typeof value}`);
  }

  return value.map((item, index) => {
    if (itemGuard(item)) {
      return item;
    }
    throw new Error(`${errorMessage} at index ${index}. Received: ${JSON.stringify(item)}`);
  });
}

export function validateObjectProperties<T extends Record<string, unknown>>(
  value: unknown,
  propertyGuards: { [K in keyof T]: (value: unknown) => value is T[K] },
  errorMessage: string
): T {
  if (!isObject(value)) {
    throw new Error(`Expected object, received: ${typeof value}`);
  }

  const result: Partial<T> = {};

  for (const [key, guard] of Object.entries(propertyGuards)) {
    const propertyValue = (value as any)[key];
    if (guard(propertyValue)) {
      (result as any)[key] = propertyValue;
    } else {
      throw new Error(`${errorMessage}: property '${key}' validation failed. Received: ${JSON.stringify(propertyValue)}`);
    }
  }

  return result as T;
}

// Runtime validation decorators (for methods)
export function validateParams(...guards: Array<(value: unknown) => boolean>) {
  return function (target: any, propertyKey: string, descriptor: PropertyDescriptor) {
    const originalMethod = descriptor.value;

    descriptor.value = function (...args: any[]) {
      guards.forEach((guard, index) => {
        if (!guard(args[index])) {
          throw new Error(`Parameter ${index} validation failed in ${propertyKey}`);
        }
      });

      return originalMethod.apply(this, args);
    };

    return descriptor;
  };
}

export function validateResult<T>(guard: (value: unknown) => value is T) {
  return function (target: any, propertyKey: string, descriptor: PropertyDescriptor) {
    const originalMethod = descriptor.value;

    descriptor.value = function (...args: any[]) {
      const result = originalMethod.apply(this, args);

      if (!guard(result)) {
        throw new Error(`Return value validation failed in ${propertyKey}`);
      }

      return result;
    };

    return descriptor;
  };
}

// Export all type guards
export const typeGuards = {
  // Primitives
  isString,
  isNumber,
  isBoolean,
  isArray,
  isObject,
  isNonEmptyString,
  isPositiveNumber,
  isNonNegativeNumber,
  isInteger,
  isFiniteNumber,

  // Domain types
  isSymbolId,
  isTimestamp,
  isPrice,
  isVolume,
  isPercentage,
  isColumn,
  isErrorSeverity,
  isErrorCategory,

  // Complex objects
  isChartStateConfig,
  isMarketData,
  isStoreState,
  isPerformanceMetrics,
  isAppError,
  isDataFetchRequest,
  isDataFetchResponse,
  isConfigurationObject,

  // Utilities
  validateAndTransform,
  validateArrayItems,
  validateObjectProperties
};