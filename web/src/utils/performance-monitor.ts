/**
 * Performance Monitoring Utilities
 * 
 * Provides comprehensive performance monitoring for the React-Rust store integration,
 * including memory tracking, FPS monitoring, and operation profiling.
 */

import { PERFORMANCE_THRESHOLDS, FEATURE_FLAGS } from '../config/store-constants';

export interface PerformanceMetric {
  name: string;
  value: number;
  timestamp: number;
  threshold?: number;
  unit: string;
  trend?: 'increasing' | 'stable' | 'decreasing';
}

export interface PerformanceReport {
  metrics: PerformanceMetric[];
  summary: {
    totalMetrics: number;
    warningCount: number;
    criticalCount: number;
    averageLatency: number;
    memoryUsage: number;
    fps: number;
  };
  recommendations: string[];
  timestamp: number;
}

export interface ProfiledOperation {
  name: string;
  startTime: number;
  endTime?: number;
  duration?: number;
  metadata?: Record<string, any>;
}

/**
 * Performance Monitor class for tracking and analyzing performance metrics
 */
export class PerformanceMonitor {
  private metrics: PerformanceMetric[] = [];
  private operations: Map<string, ProfiledOperation> = new Map();
  private fpsHistory: number[] = [];
  private memoryHistory: number[] = [];
  private enabled: boolean = FEATURE_FLAGS.ENABLE_PERFORMANCE_MONITORING;
  private maxHistorySize: number = 100;
  private lastCleanup: number = 0;
  private cleanupInterval: number = 60000; // 1 minute

  constructor() {
    if (this.enabled) {
      this.startMonitoring();
    }
  }

  /**
   * Start automatic performance monitoring
   */
  private startMonitoring(): void {
    // FPS monitoring
    this.startFpsMonitoring();
    
    // Memory monitoring
    this.startMemoryMonitoring();
    
    // Cleanup old metrics periodically
    setInterval(() => this.cleanup(), this.cleanupInterval);
  }

  /**
   * Start FPS monitoring using requestAnimationFrame
   */
  private startFpsMonitoring(): void {
    let lastTime = performance.now();
    let frameCount = 0;
    const fpsInterval = 1000; // Calculate FPS every second

    const measureFps = (currentTime: number) => {
      frameCount++;
      const elapsed = currentTime - lastTime;

      if (elapsed >= fpsInterval) {
        const fps = Math.round((frameCount * 1000) / elapsed);
        this.recordMetric('fps', fps, PERFORMANCE_THRESHOLDS.MIN_FPS, 'fps');
        
        this.fpsHistory.push(fps);
        if (this.fpsHistory.length > this.maxHistorySize) {
          this.fpsHistory.shift();
        }

        frameCount = 0;
        lastTime = currentTime;
      }

      requestAnimationFrame(measureFps);
    };

    requestAnimationFrame(measureFps);
  }

  /**
   * Start memory usage monitoring
   */
  private startMemoryMonitoring(): void {
    const measureMemory = () => {
      if (performance.memory) {
        const memoryUsage = performance.memory.usedJSHeapSize;
        this.recordMetric(
          'memory_usage', 
          memoryUsage, 
          PERFORMANCE_THRESHOLDS.MEMORY_WARNING_THRESHOLD, 
          'bytes'
        );
        
        this.memoryHistory.push(memoryUsage);
        if (this.memoryHistory.length > this.maxHistorySize) {
          this.memoryHistory.shift();
        }
      }
    };

    // Measure memory every 5 seconds
    setInterval(measureMemory, 5000);
    measureMemory(); // Initial measurement
  }

  /**
   * Record a performance metric
   */
  recordMetric(
    name: string, 
    value: number, 
    threshold?: number, 
    unit: string = 'ms',
    metadata?: Record<string, any>
  ): void {
    if (!this.enabled) return;

    const metric: PerformanceMetric = {
      name,
      value,
      timestamp: performance.now(),
      threshold,
      unit,
      trend: this.calculateTrend(name, value),
    };

    this.metrics.push(metric);

    // Limit metrics history
    if (this.metrics.length > this.maxHistorySize * 10) {
      this.metrics = this.metrics.slice(-this.maxHistorySize * 5);
    }

    // Check for performance issues
    this.checkThresholds(metric);
  }

  /**
   * Start profiling an operation
   */
  startProfile(operationName: string, metadata?: Record<string, any>): void {
    if (!this.enabled) return;

    const operation: ProfiledOperation = {
      name: operationName,
      startTime: performance.now(),
      metadata,
    };

    this.operations.set(operationName, operation);
  }

  /**
   * End profiling an operation and record the duration
   */
  endProfile(operationName: string): number | null {
    if (!this.enabled) return null;

    const operation = this.operations.get(operationName);
    if (!operation) {
      console.warn(`Performance profile not found: ${operationName}`);
      return null;
    }

    operation.endTime = performance.now();
    operation.duration = operation.endTime - operation.startTime;

    // Record as metric
    this.recordMetric(
      `operation_${operationName}`,
      operation.duration,
      this.getOperationThreshold(operationName),
      'ms',
      operation.metadata
    );

    this.operations.delete(operationName);
    return operation.duration;
  }

  /**
   * Profile a function execution
   */
  async profileAsync<T>(
    operationName: string,
    fn: () => Promise<T>,
    metadata?: Record<string, any>
  ): Promise<T> {
    this.startProfile(operationName, metadata);
    try {
      const result = await fn();
      return result;
    } finally {
      this.endProfile(operationName);
    }
  }

  /**
   * Profile a synchronous function execution
   */
  profile<T>(
    operationName: string,
    fn: () => T,
    metadata?: Record<string, any>
  ): T {
    this.startProfile(operationName, metadata);
    try {
      return fn();
    } finally {
      this.endProfile(operationName);
    }
  }

  /**
   * Get performance report
   */
  getReport(): PerformanceReport {
    const recentMetrics = this.metrics.slice(-100); // Last 100 metrics
    
    const warningCount = recentMetrics.filter(m => 
      m.threshold && m.value > m.threshold
    ).length;
    
    const criticalCount = recentMetrics.filter(m =>
      m.threshold && m.value > m.threshold * 2
    ).length;

    const latencyMetrics = recentMetrics.filter(m => m.unit === 'ms');
    const averageLatency = latencyMetrics.length > 0
      ? latencyMetrics.reduce((sum, m) => sum + m.value, 0) / latencyMetrics.length
      : 0;

    const currentFps = this.fpsHistory.length > 0 
      ? this.fpsHistory[this.fpsHistory.length - 1] 
      : 0;

    const currentMemory = this.memoryHistory.length > 0
      ? this.memoryHistory[this.memoryHistory.length - 1]
      : 0;

    const recommendations = this.generateRecommendations(recentMetrics);

    return {
      metrics: recentMetrics,
      summary: {
        totalMetrics: recentMetrics.length,
        warningCount,
        criticalCount,
        averageLatency,
        memoryUsage: currentMemory,
        fps: currentFps,
      },
      recommendations,
      timestamp: Date.now(),
    };
  }

  /**
   * Calculate trend for a metric
   */
  private calculateTrend(metricName: string, currentValue: number): 'increasing' | 'stable' | 'decreasing' {
    const recentMetrics = this.metrics
      .filter(m => m.name === metricName)
      .slice(-5); // Last 5 values

    if (recentMetrics.length < 3) return 'stable';

    const values = recentMetrics.map(m => m.value);
    const first = values[0];
    const last = currentValue;
    const threshold = first * 0.1; // 10% change threshold

    if (last > first + threshold) return 'increasing';
    if (last < first - threshold) return 'decreasing';
    return 'stable';
  }

  /**
   * Get threshold for specific operations
   */
  private getOperationThreshold(operationName: string): number {
    const thresholds: Record<string, number> = {
      'store_update': PERFORMANCE_THRESHOLDS.MAX_STATE_UPDATE_LATENCY,
      'wasm_call': 5, // 5ms for WASM calls
      'validation': PERFORMANCE_THRESHOLDS.MAX_VALIDATION_LATENCY,
      'serialization': 10, // 10ms for serialization
      'render': PERFORMANCE_THRESHOLDS.MAX_RENDER_LATENCY,
    };

    return thresholds[operationName] || 50; // Default 50ms
  }

  /**
   * Check if metrics exceed thresholds and log warnings
   */
  private checkThresholds(metric: PerformanceMetric): void {
    if (!metric.threshold) return;

    if (metric.value > metric.threshold * 2) {
      console.error(`[Performance] CRITICAL: ${metric.name} = ${metric.value}${metric.unit} (threshold: ${metric.threshold}${metric.unit})`);
    } else if (metric.value > metric.threshold) {
      console.warn(`[Performance] WARNING: ${metric.name} = ${metric.value}${metric.unit} (threshold: ${metric.threshold}${metric.unit})`);
    }
  }

  /**
   * Generate performance recommendations
   */
  private generateRecommendations(metrics: PerformanceMetric[]): string[] {
    const recommendations: string[] = [];

    // Check FPS
    const fpsMetrics = metrics.filter(m => m.name === 'fps');
    if (fpsMetrics.length > 0) {
      const avgFps = fpsMetrics.reduce((sum, m) => sum + m.value, 0) / fpsMetrics.length;
      if (avgFps < PERFORMANCE_THRESHOLDS.MIN_FPS) {
        recommendations.push('Low FPS detected. Consider reducing visual complexity or enabling performance optimizations.');
      }
    }

    // Check memory usage
    const memoryMetrics = metrics.filter(m => m.name === 'memory_usage');
    if (memoryMetrics.length > 0) {
      const maxMemory = Math.max(...memoryMetrics.map(m => m.value));
      if (maxMemory > PERFORMANCE_THRESHOLDS.MEMORY_CRITICAL_THRESHOLD) {
        recommendations.push('High memory usage detected. Consider implementing memory cleanup or reducing cache size.');
      }
    }

    // Check operation latencies
    const latencyMetrics = metrics.filter(m => m.unit === 'ms' && m.value > 50);
    if (latencyMetrics.length > 5) {
      recommendations.push('Multiple slow operations detected. Consider optimizing store updates or WASM calls.');
    }

    // Check error patterns
    const errorMetrics = metrics.filter(m => m.name.includes('error'));
    if (errorMetrics.length > 3) {
      recommendations.push('High error rate detected. Review error handling and recovery strategies.');
    }

    return recommendations;
  }

  /**
   * Cleanup old metrics and operations
   */
  private cleanup(): void {
    const now = performance.now();
    
    // Remove metrics older than 5 minutes
    const fiveMinutesAgo = now - 300000;
    this.metrics = this.metrics.filter(m => m.timestamp > fiveMinutesAgo);
    
    // Remove stale operations (longer than 30 seconds)
    const thirtySecondsAgo = now - 30000;
    for (const [name, operation] of this.operations.entries()) {
      if (operation.startTime < thirtySecondsAgo) {
        console.warn(`[Performance] Stale operation detected: ${name}`);
        this.operations.delete(name);
      }
    }

    this.lastCleanup = now;
  }

  /**
   * Enable or disable monitoring
   */
  setEnabled(enabled: boolean): void {
    this.enabled = enabled;
    if (enabled && !FEATURE_FLAGS.ENABLE_PERFORMANCE_MONITORING) {
      console.warn('[Performance] Monitoring enabled but feature flag is disabled');
    }
  }

  /**
   * Clear all metrics and reset
   */
  reset(): void {
    this.metrics = [];
    this.operations.clear();
    this.fpsHistory = [];
    this.memoryHistory = [];
  }

  /**
   * Get current FPS
   */
  getCurrentFps(): number {
    return this.fpsHistory.length > 0 ? this.fpsHistory[this.fpsHistory.length - 1] : 0;
  }

  /**
   * Get current memory usage
   */
  getCurrentMemoryUsage(): number {
    return this.memoryHistory.length > 0 ? this.memoryHistory[this.memoryHistory.length - 1] : 0;
  }

  /**
   * Get metrics by name
   */
  getMetricsByName(name: string): PerformanceMetric[] {
    return this.metrics.filter(m => m.name === name);
  }
}

// Global performance monitor instance
export const performanceMonitor = new PerformanceMonitor();

// Utility functions for easy access
export const profile = performanceMonitor.profile.bind(performanceMonitor);
export const profileAsync = performanceMonitor.profileAsync.bind(performanceMonitor);
export const recordMetric = performanceMonitor.recordMetric.bind(performanceMonitor);
export const getPerformanceReport = performanceMonitor.getReport.bind(performanceMonitor);