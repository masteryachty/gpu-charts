import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

/**
 * Performance monitoring configuration
 */
export interface PerformanceMonitorConfig {
  /** Enable FPS monitoring */
  enableFpsMonitoring?: boolean;
  
  /** Enable memory usage monitoring */
  enableMemoryMonitoring?: boolean;
  
  /** Enable CPU usage estimation */
  enableCpuMonitoring?: boolean;
  
  /** Update interval in milliseconds */
  updateIntervalMs?: number;
  
  /** Maximum history size for metrics */
  maxHistorySize?: number;
  
  /** Enable performance warnings */
  enableWarnings?: boolean;
  
  /** Minimum FPS threshold for warnings */
  fpsWarningThreshold?: number;
  
  /** Memory warning threshold in bytes */
  memoryWarningThreshold?: number;
}

/**
 * Real-time performance metrics
 */
export interface PerformanceMetrics {
  /** Current frames per second */
  fps: number;
  
  /** Average FPS over recent history */
  avgFps: number;
  
  /** Total memory usage in bytes */
  totalMemoryUsage: number;
  
  /** JavaScript heap usage in bytes */
  heapUsage: number;
  
  /** Estimated CPU usage percentage */
  cpuUsage: number;
  
  /** Last update timestamp */
  lastUpdate: number;
  
  /** Frame render latency in milliseconds */
  renderLatency: number;
  
  /** Number of performance warnings */
  warningCount: number;
  
  /** Is performance degraded */
  isDegraded: boolean;
  
  /** Performance score (0-100) */
  performanceScore: number;
}

/**
 * Performance warning types
 */
export interface PerformanceWarning {
  type: 'fps' | 'memory' | 'cpu' | 'latency';
  message: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  timestamp: number;
  value: number;
  threshold: number;
}

/**
 * Performance monitor state
 */
export interface PerformanceMonitorState {
  metrics: PerformanceMetrics;
  isMonitoring: boolean;
  warnings: PerformanceWarning[];
  history: {
    fps: number[];
    memory: number[];
    cpu: number[];
    timestamps: number[];
  };
}

/**
 * Performance monitor API
 */
export interface PerformanceMonitorAPI {
  /** Start monitoring */
  start: () => void;
  
  /** Stop monitoring */
  stop: () => void;
  
  /** Reset metrics and history */
  reset: () => void;
  
  /** Get current metrics snapshot */
  getMetrics: () => PerformanceMetrics;
  
  /** Get performance warnings */
  getWarnings: () => PerformanceWarning[];
  
  /** Clear warnings */
  clearWarnings: () => void;
  
  /** Update configuration */
  updateConfig: (config: Partial<PerformanceMonitorConfig>) => void;
}

/**
 * Default configuration
 */
const DEFAULT_CONFIG: Required<PerformanceMonitorConfig> = {
  enableFpsMonitoring: true,
  enableMemoryMonitoring: true,
  enableCpuMonitoring: true,
  updateIntervalMs: 1000, // Update every second
  maxHistorySize: 60, // Keep 60 seconds of history
  enableWarnings: true,
  fpsWarningThreshold: 30,
  memoryWarningThreshold: 100 * 1024 * 1024, // 100MB
};

/**
 * Advanced performance monitoring hook
 */
export function usePerformanceMonitor(
  config: PerformanceMonitorConfig = {}
): [PerformanceMonitorState, PerformanceMonitorAPI] {
  const mergedConfig = { ...DEFAULT_CONFIG, ...config };
  
  // State management
  const [state, setState] = useState<PerformanceMonitorState>({
    metrics: {
      fps: 0,
      avgFps: 0,
      totalMemoryUsage: 0,
      heapUsage: 0,
      cpuUsage: 0,
      lastUpdate: 0,
      renderLatency: 0,
      warningCount: 0,
      isDegraded: false,
      performanceScore: 100,
    },
    isMonitoring: false,
    warnings: [],
    history: {
      fps: [],
      memory: [],
      cpu: [],
      timestamps: [],
    },
  });
  
  // Refs for monitoring
  const intervalRef = useRef<ReturnType<typeof setInterval>>();
  const frameCountRef = useRef(0);
  const lastFrameTimeRef = useRef(performance.now());
  const lastCpuTimeRef = useRef(performance.now());
  const configRef = useRef(mergedConfig);
  
  // Update config ref when config changes
  useEffect(() => {
    configRef.current = { ...DEFAULT_CONFIG, ...config };
  }, [config]);
  
  /**
   * Measure FPS using requestAnimationFrame
   */
  const measureFps = useCallback(() => {
    const currentTime = performance.now();
    const deltaTime = currentTime - lastFrameTimeRef.current;
    
    if (deltaTime > 0) {
      const fps = Math.round(1000 / deltaTime);
      frameCountRef.current++;
      lastFrameTimeRef.current = currentTime;
      return Math.min(fps, 120); // Cap at 120 FPS
    }
    
    return 0;
  }, []);
  
  /**
   * Measure memory usage
   */
  const measureMemory = useCallback(() => {
    if ('memory' in performance) {
      const memory = (performance as any).memory;
      return {
        totalMemoryUsage: memory.totalJSHeapSize || 50 * 1024 * 1024,
        heapUsage: memory.usedJSHeapSize || 25 * 1024 * 1024,
      };
    }
    
    // Fallback estimates
    return {
      totalMemoryUsage: 50 * 1024 * 1024, // 50MB
      heapUsage: 25 * 1024 * 1024, // 25MB
    };
  }, []);
  
  /**
   * Estimate CPU usage
   */
  const measureCpu = useCallback(() => {
    const currentTime = performance.now();
    const deltaTime = currentTime - lastCpuTimeRef.current;
    lastCpuTimeRef.current = currentTime;
    
    // Simple CPU estimation based on frame timing
    if (deltaTime > 16.67) { // 60fps threshold
      const cpuUsage = Math.min(((deltaTime - 16.67) / 16.67) * 100, 100);
      return Math.round(cpuUsage);
    }
    
    return 0;
  }, []);
  
  /**
   * Calculate performance score
   */
  const calculatePerformanceScore = useCallback((metrics: PerformanceMetrics): number => {
    let score = 100;
    
    // FPS impact
    if (metrics.fps < 60) score -= (60 - metrics.fps) * 1.5;
    if (metrics.fps < 30) score -= 20;
    
    // Memory impact
    const memoryMB = metrics.totalMemoryUsage / (1024 * 1024);
    if (memoryMB > 100) score -= (memoryMB - 100) * 0.5;
    if (memoryMB > 200) score -= 20;
    
    // CPU impact
    if (metrics.cpuUsage > 50) score -= (metrics.cpuUsage - 50) * 0.8;
    if (metrics.cpuUsage > 80) score -= 15;
    
    // Latency impact
    if (metrics.renderLatency > 16) score -= (metrics.renderLatency - 16) * 2;
    if (metrics.renderLatency > 50) score -= 25;
    
    return Math.max(score, 0);
  }, []);
  
  /**
   * Check for performance warnings
   */
  const checkWarnings = useCallback((metrics: PerformanceMetrics): PerformanceWarning[] => {
    const warnings: PerformanceWarning[] = [];
    const timestamp = Date.now();
    
    if (configRef.current.enableWarnings) {
      // FPS warnings
      if (configRef.current.enableFpsMonitoring && metrics.fps < configRef.current.fpsWarningThreshold) {
        warnings.push({
          type: 'fps',
          message: `Low FPS detected: ${metrics.fps} (threshold: ${configRef.current.fpsWarningThreshold})`,
          severity: metrics.fps < 15 ? 'critical' : metrics.fps < 24 ? 'high' : 'medium',
          timestamp,
          value: metrics.fps,
          threshold: configRef.current.fpsWarningThreshold,
        });
      }
      
      // Memory warnings
      if (configRef.current.enableMemoryMonitoring && metrics.totalMemoryUsage > configRef.current.memoryWarningThreshold) {
        const memoryMB = Math.round(metrics.totalMemoryUsage / (1024 * 1024));
        const thresholdMB = Math.round(configRef.current.memoryWarningThreshold / (1024 * 1024));
        warnings.push({
          type: 'memory',
          message: `High memory usage: ${memoryMB}MB (threshold: ${thresholdMB}MB)`,
          severity: memoryMB > thresholdMB * 2 ? 'critical' : memoryMB > thresholdMB * 1.5 ? 'high' : 'medium',
          timestamp,
          value: metrics.totalMemoryUsage,
          threshold: configRef.current.memoryWarningThreshold,
        });
      }
      
      // CPU warnings
      if (configRef.current.enableCpuMonitoring && metrics.cpuUsage > 70) {
        warnings.push({
          type: 'cpu',
          message: `High CPU usage: ${metrics.cpuUsage}% (threshold: 70%)`,
          severity: metrics.cpuUsage > 90 ? 'critical' : metrics.cpuUsage > 80 ? 'high' : 'medium',
          timestamp,
          value: metrics.cpuUsage,
          threshold: 70,
        });
      }
      
      // Latency warnings
      if (metrics.renderLatency > 50) {
        warnings.push({
          type: 'latency',
          message: `High render latency: ${metrics.renderLatency}ms (threshold: 50ms)`,
          severity: metrics.renderLatency > 100 ? 'critical' : metrics.renderLatency > 75 ? 'high' : 'medium',
          timestamp,
          value: metrics.renderLatency,
          threshold: 50,
        });
      }
    }
    
    return warnings;
  }, []);
  
  /**
   * Update metrics
   */
  const updateMetrics = useCallback(() => {
    const currentTime = Date.now();
    const fps = configRef.current.enableFpsMonitoring ? measureFps() : 0;
    const memory = configRef.current.enableMemoryMonitoring ? measureMemory() : { totalMemoryUsage: 0, heapUsage: 0 };
    const cpuUsage = configRef.current.enableCpuMonitoring ? measureCpu() : 0;
    const renderLatency = currentTime - lastFrameTimeRef.current;
    
    setState(prevState => {
      // Update history
      const newFpsHistory = [...prevState.history.fps, fps].slice(-configRef.current.maxHistorySize);
      const newMemoryHistory = [...prevState.history.memory, memory.totalMemoryUsage].slice(-configRef.current.maxHistorySize);
      const newCpuHistory = [...prevState.history.cpu, cpuUsage].slice(-configRef.current.maxHistorySize);
      const newTimestamps = [...prevState.history.timestamps, currentTime].slice(-configRef.current.maxHistorySize);
      
      // Calculate average FPS
      const avgFps = newFpsHistory.length > 0 ? 
        Math.round(newFpsHistory.reduce((a, b) => a + b, 0) / newFpsHistory.length) : 0;
      
      // Create new metrics
      const newMetrics: PerformanceMetrics = {
        fps,
        avgFps,
        totalMemoryUsage: memory.totalMemoryUsage,
        heapUsage: memory.heapUsage,
        cpuUsage,
        lastUpdate: currentTime,
        renderLatency: Math.round(renderLatency),
        warningCount: prevState.warnings.length,
        isDegraded: fps < 30 || memory.totalMemoryUsage > configRef.current.memoryWarningThreshold || cpuUsage > 70,
        performanceScore: 0, // Will be calculated below
      };
      
      // Calculate performance score
      newMetrics.performanceScore = calculatePerformanceScore(newMetrics);
      
      // Check for new warnings
      const newWarnings = checkWarnings(newMetrics);
      const allWarnings = [...prevState.warnings, ...newWarnings].slice(-50); // Keep last 50 warnings
      
      return {
        ...prevState,
        metrics: { ...newMetrics, warningCount: allWarnings.length },
        warnings: allWarnings,
        history: {
          fps: newFpsHistory,
          memory: newMemoryHistory,
          cpu: newCpuHistory,
          timestamps: newTimestamps,
        },
      };
    });
  }, [measureFps, measureMemory, measureCpu, calculatePerformanceScore, checkWarnings]);
  
  /**
   * Start monitoring
   */
  const start = useCallback(() => {
    if (intervalRef.current) return; // Already running
    
    setState(prev => ({ ...prev, isMonitoring: true }));
    
    intervalRef.current = setInterval(() => {
      updateMetrics();
    }, configRef.current.updateIntervalMs);
    
    // console.log('[usePerformanceMonitor] Started monitoring');
  }, [updateMetrics]);
  
  /**
   * Stop monitoring
   */
  const stop = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = undefined;
    }
    
    setState(prev => ({ ...prev, isMonitoring: false }));
    // console.log('[usePerformanceMonitor] Stopped monitoring');
  }, []);
  
  /**
   * Reset metrics and history
   */
  const reset = useCallback(() => {
    setState(prev => ({
      ...prev,
      metrics: {
        fps: 0,
        avgFps: 0,
        totalMemoryUsage: 0,
        heapUsage: 0,
        cpuUsage: 0,
        lastUpdate: 0,
        renderLatency: 0,
        warningCount: 0,
        isDegraded: false,
        performanceScore: 100,
      },
      warnings: [],
      history: {
        fps: [],
        memory: [],
        cpu: [],
        timestamps: [],
      },
    }));
    
    frameCountRef.current = 0;
    lastFrameTimeRef.current = performance.now();
    lastCpuTimeRef.current = performance.now();
    
    console.log('[usePerformanceMonitor] Reset metrics');
  }, []);
  
  /**
   * Get current metrics
   */
  const getMetrics = useCallback((): PerformanceMetrics => {
    return state.metrics;
  }, [state.metrics]);
  
  /**
   * Get warnings
   */
  const getWarnings = useCallback((): PerformanceWarning[] => {
    return state.warnings;
  }, [state.warnings]);
  
  /**
   * Clear warnings
   */
  const clearWarnings = useCallback(() => {
    setState(prev => ({
      ...prev,
      warnings: [],
      metrics: { ...prev.metrics, warningCount: 0 },
    }));
  }, []);
  
  /**
   * Update configuration
   */
  const updateConfig = useCallback((newConfig: Partial<PerformanceMonitorConfig>) => {
    configRef.current = { ...configRef.current, ...newConfig };
    
    // Restart monitoring if interval changed
    if (newConfig.updateIntervalMs && state.isMonitoring) {
      stop();
      setTimeout(start, 100);
    }
  }, [state.isMonitoring, start, stop]);
  
  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, []);
  
  // Expose metrics globally for testing
  useEffect(() => {
    if (typeof window !== 'undefined') {
      (window as any).__PERFORMANCE_METRICS__ = state.metrics;
      (window as any).__PERFORMANCE_MONITOR_STATE__ = state;
      (window as any).__GET_PERFORMANCE_METRICS__ = getMetrics;
    }
  }, [state, getMetrics]);
  
  // API object - memoized to prevent unnecessary re-renders
  const api: PerformanceMonitorAPI = useMemo(() => ({
    start,
    stop,
    reset,
    getMetrics,
    getWarnings,
    clearWarnings,
    updateConfig,
  }), [start, stop, reset, getMetrics, getWarnings, clearWarnings, updateConfig]);
  
  return [state, api];
}

// Types are already exported above, no need to re-export