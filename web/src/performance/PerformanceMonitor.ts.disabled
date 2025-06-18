/**
 * Comprehensive Performance Monitoring System
 * 
 * Monitors and optimizes performance across the React-Rust integration,
 * including memory usage, frame rates, WebGPU metrics, and data processing.
 */

import { handlePerformanceError, ERROR_CODES } from '../errors';

export interface PerformanceMetrics {
  // Rendering performance
  fps: number;
  frameTime: number; // milliseconds
  renderLatency: number; // milliseconds
  droppedFrames: number;
  
  // Memory usage
  jsHeapUsed: number; // bytes
  jsHeapTotal: number; // bytes
  wasmMemoryUsed: number; // bytes
  totalMemoryUsage: number; // bytes
  memoryTrend: 'increasing' | 'stable' | 'decreasing';
  
  // Data processing
  dataProcessingTime: number; // milliseconds
  dataTransferTime: number; // milliseconds
  cacheHitRate: number; // percentage
  
  // Network performance
  networkLatency: number; // milliseconds
  bandwidth: number; // bytes per second
  packetLoss: number; // percentage
  
  // CPU utilization
  cpuUsage: number; // percentage (estimated)
  mainThreadBlockTime: number; // milliseconds
  
  // WebGPU specific
  webgpuMemoryUsage: number; // bytes
  gpuUtilization: number; // percentage (estimated)
  shaderCompileTime: number; // milliseconds
  
  // System health
  timestamp: number;
  systemHealth: 'excellent' | 'good' | 'fair' | 'poor' | 'critical';
}

export interface PerformanceThresholds {
  fps: { warning: number; critical: number };
  frameTime: { warning: number; critical: number };
  memoryUsage: { warning: number; critical: number };
  cpuUsage: { warning: number; critical: number };
  networkLatency: { warning: number; critical: number };
}

export interface PerformanceOptimization {
  id: string;
  name: string;
  description: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  action: () => Promise<boolean>;
  conditions: (metrics: PerformanceMetrics) => boolean;
  enabled: boolean;
}

export class PerformanceMonitor {
  private metrics: PerformanceMetrics;
  private metricsHistory: PerformanceMetrics[] = [];
  private subscribers: Array<(metrics: PerformanceMetrics) => void> = [];
  private optimizations: Map<string, PerformanceOptimization> = new Map();
  private monitoringInterval: NodeJS.Timeout | null = null;
  private isMonitoring = false;
  
  private thresholds: PerformanceThresholds = {
    fps: { warning: 30, critical: 15 },
    frameTime: { warning: 33, critical: 66 }, // 30fps = 33ms, 15fps = 66ms
    memoryUsage: { warning: 500 * 1024 * 1024, critical: 1024 * 1024 * 1024 }, // 500MB, 1GB
    cpuUsage: { warning: 70, critical: 90 },
    networkLatency: { warning: 1000, critical: 3000 }
  };
  
  constructor() {
    this.metrics = this.createInitialMetrics();
    this.setupOptimizations();
  }
  
  /**
   * Start performance monitoring
   */
  startMonitoring(intervalMs: number = 1000): void {
    if (this.isMonitoring) {
      console.warn('[PerformanceMonitor] Already monitoring');
      return;
    }
    
    console.log('[PerformanceMonitor] Starting performance monitoring');
    this.isMonitoring = true;
    
    this.monitoringInterval = setInterval(async () => {
      await this.collectMetrics();
      this.analyzePerformance();
      this.notifySubscribers();
    }, intervalMs);
  }
  
  /**
   * Stop performance monitoring
   */
  stopMonitoring(): void {
    if (!this.isMonitoring) return;
    
    console.log('[PerformanceMonitor] Stopping performance monitoring');
    this.isMonitoring = false;
    
    if (this.monitoringInterval) {
      clearInterval(this.monitoringInterval);
      this.monitoringInterval = null;
    }
  }
  
  /**
   * Get current performance metrics
   */
  getCurrentMetrics(): PerformanceMetrics {
    return { ...this.metrics };
  }
  
  /**
   * Get performance metrics history
   */
  getMetricsHistory(durationMs?: number): PerformanceMetrics[] {
    if (!durationMs) return [...this.metricsHistory];
    
    const cutoff = Date.now() - durationMs;
    return this.metricsHistory.filter(m => m.timestamp >= cutoff);
  }
  
  /**
   * Subscribe to performance metrics updates
   */
  subscribe(callback: (metrics: PerformanceMetrics) => void): () => void {
    this.subscribers.push(callback);
    return () => {
      const index = this.subscribers.indexOf(callback);
      if (index >= 0) {
        this.subscribers.splice(index, 1);
      }
    };
  }
  
  /**
   * Register performance optimization
   */
  registerOptimization(optimization: PerformanceOptimization): void {
    this.optimizations.set(optimization.id, optimization);
    console.log(`[PerformanceMonitor] Registered optimization: ${optimization.name}`);
  }
  
  /**
   * Trigger manual performance optimization
   */
  async optimizePerformance(): Promise<string[]> {
    const appliedOptimizations: string[] = [];
    
    for (const [id, optimization] of this.optimizations) {
      if (!optimization.enabled) continue;
      
      if (optimization.conditions(this.metrics)) {
        console.log(`[PerformanceMonitor] Applying optimization: ${optimization.name}`);
        
        try {
          const success = await optimization.action();
          if (success) {
            appliedOptimizations.push(optimization.name);
            console.log(`[PerformanceMonitor] Successfully applied: ${optimization.name}`);
          } else {
            console.warn(`[PerformanceMonitor] Failed to apply: ${optimization.name}`);
          }
        } catch (error) {
          console.error(`[PerformanceMonitor] Error applying ${optimization.name}:`, error);
        }
      }
    }
    
    return appliedOptimizations;
  }
  
  /**
   * Get performance recommendations
   */
  getRecommendations(): Array<{ severity: string; message: string; action?: string }> {
    const recommendations: Array<{ severity: string; message: string; action?: string }> = [];
    const metrics = this.metrics;
    
    // FPS recommendations
    if (metrics.fps < this.thresholds.fps.critical) {
      recommendations.push({
        severity: 'critical',
        message: `Very low frame rate (${metrics.fps.toFixed(1)} FPS)`,
        action: 'Reduce chart complexity or enable performance mode'
      });
    } else if (metrics.fps < this.thresholds.fps.warning) {
      recommendations.push({
        severity: 'warning',
        message: `Low frame rate (${metrics.fps.toFixed(1)} FPS)`,
        action: 'Consider reducing visual effects'
      });
    }
    
    // Memory recommendations
    if (metrics.totalMemoryUsage > this.thresholds.memoryUsage.critical) {
      recommendations.push({
        severity: 'critical',
        message: `High memory usage (${(metrics.totalMemoryUsage / 1024 / 1024).toFixed(1)} MB)`,
        action: 'Clear cache or restart application'
      });
    } else if (metrics.totalMemoryUsage > this.thresholds.memoryUsage.warning) {
      recommendations.push({
        severity: 'warning',
        message: `Elevated memory usage (${(metrics.totalMemoryUsage / 1024 / 1024).toFixed(1)} MB)`,
        action: 'Monitor for memory leaks'
      });
    }
    
    // CPU recommendations
    if (metrics.cpuUsage > this.thresholds.cpuUsage.critical) {
      recommendations.push({
        severity: 'critical',
        message: `High CPU usage (${metrics.cpuUsage.toFixed(1)}%)`,
        action: 'Reduce computational load'
      });
    }
    
    // Network recommendations
    if (metrics.networkLatency > this.thresholds.networkLatency.critical) {
      recommendations.push({
        severity: 'critical',
        message: `High network latency (${metrics.networkLatency.toFixed(0)}ms)`,
        action: 'Check network connection'
      });
    }
    
    return recommendations;
  }
  
  /**
   * Collect comprehensive performance metrics
   */
  private async collectMetrics(): Promise<void> {
    const startTime = performance.now();
    
    try {
      // JavaScript memory metrics
      const jsMemory = this.getJavaScriptMemory();
      
      // WASM memory metrics
      const wasmMemory = await this.getWasmMemory();
      
      // Rendering metrics
      const renderingMetrics = this.getRenderingMetrics();
      
      // Network metrics
      const networkMetrics = await this.getNetworkMetrics();
      
      // System metrics
      const systemMetrics = this.getSystemMetrics();
      
      // WebGPU metrics
      const webgpuMetrics = await this.getWebGPUMetrics();
      
      const totalMemory = jsMemory.used + wasmMemory.used;
      const memoryTrend = this.calculateMemoryTrend(totalMemory);
      
      // Update metrics
      this.metrics = {
        ...renderingMetrics,
        ...networkMetrics,
        ...systemMetrics,
        ...webgpuMetrics,
        jsHeapUsed: jsMemory.used,
        jsHeapTotal: jsMemory.total,
        wasmMemoryUsed: wasmMemory.used,
        totalMemoryUsage: totalMemory,
        memoryTrend,
        timestamp: Date.now(),
        systemHealth: this.calculateSystemHealth()
      };
      
      // Add to history
      this.metricsHistory.push({ ...this.metrics });
      
      // Maintain history size (keep last 1000 entries)
      if (this.metricsHistory.length > 1000) {
        this.metricsHistory = this.metricsHistory.slice(-1000);
      }
      
    } catch (error) {
      console.error('[PerformanceMonitor] Error collecting metrics:', error);
    }
  }
  
  /**
   * Get JavaScript memory usage
   */
  private getJavaScriptMemory(): { used: number; total: number } {
    if ('memory' in performance) {
      const memory = (performance as any).memory;
      return {
        used: memory.usedJSHeapSize || 0,
        total: memory.totalJSHeapSize || 0
      };
    }
    
    // Fallback estimation
    return { used: 0, total: 0 };
  }
  
  /**
   * Get WASM memory usage
   */
  private async getWasmMemory(): Promise<{ used: number; total: number }> {
    try {
      // This would need to be implemented in the WASM module
      // For now, return estimated values
      return { used: 0, total: 0 };
    } catch (error) {
      return { used: 0, total: 0 };
    }
  }
  
  /**
   * Get rendering performance metrics
   */
  private getRenderingMetrics(): Partial<PerformanceMetrics> {
    const now = performance.now();
    
    // Simple FPS calculation based on time since last update
    const timeSinceLastUpdate = now - (this.metrics?.timestamp || now - 16);
    const estimatedFps = timeSinceLastUpdate > 0 ? 1000 / timeSinceLastUpdate : 60;
    
    return {
      fps: Math.min(Math.max(estimatedFps, 0), 120), // Clamp between 0-120
      frameTime: timeSinceLastUpdate,
      renderLatency: 0, // Would be set by the rendering system
      droppedFrames: 0
    };
  }
  
  /**
   * Get network performance metrics
   */
  private async getNetworkMetrics(): Promise<Partial<PerformanceMetrics>> {
    try {
      // Use Navigation API if available
      if ('connection' in navigator) {
        const connection = (navigator as any).connection;
        return {
          networkLatency: connection.rtt || 0,
          bandwidth: connection.downlink ? connection.downlink * 1024 * 1024 / 8 : 0, // Convert Mbps to bytes/sec
          packetLoss: 0
        };
      }
      
      // Fallback: measure latency to our own server
      const startTime = performance.now();
      try {
        await fetch('/api/ping', { method: 'HEAD' });
        const latency = performance.now() - startTime;
        return {
          networkLatency: latency,
          bandwidth: 0,
          packetLoss: 0
        };
      } catch {
        return {
          networkLatency: 999999,
          bandwidth: 0,
          packetLoss: 100
        };
      }
    } catch (error) {
      return {
        networkLatency: 0,
        bandwidth: 0,
        packetLoss: 0
      };
    }
  }
  
  /**
   * Get system performance metrics
   */
  private getSystemMetrics(): Partial<PerformanceMetrics> {
    // Estimate CPU usage based on frame timing
    const frameTime = this.metrics?.frameTime || 16;
    const cpuUsage = Math.min((frameTime / 16) * 30, 100); // Rough estimation
    
    return {
      cpuUsage,
      mainThreadBlockTime: Math.max(frameTime - 16, 0),
      dataProcessingTime: 0, // Would be set by data processing systems
      dataTransferTime: 0,
      cacheHitRate: 0 // Would be set by caching systems
    };
  }
  
  /**
   * Get WebGPU performance metrics
   */
  private async getWebGPUMetrics(): Promise<Partial<PerformanceMetrics>> {
    try {
      // This would need integration with the WebGPU rendering system
      return {
        webgpuMemoryUsage: 0,
        gpuUtilization: 0,
        shaderCompileTime: 0
      };
    } catch (error) {
      return {
        webgpuMemoryUsage: 0,
        gpuUtilization: 0,
        shaderCompileTime: 0
      };
    }
  }
  
  /**
   * Calculate memory trend
   */
  private calculateMemoryTrend(currentMemory: number): 'increasing' | 'stable' | 'decreasing' {
    const recentHistory = this.metricsHistory.slice(-10);
    if (recentHistory.length < 5) return 'stable';
    
    const oldAverage = recentHistory.slice(0, 5).reduce((sum, m) => sum + m.totalMemoryUsage, 0) / 5;
    const newAverage = recentHistory.slice(-5).reduce((sum, m) => sum + m.totalMemoryUsage, 0) / 5;
    
    const changePercent = (newAverage - oldAverage) / oldAverage * 100;
    
    if (changePercent > 5) return 'increasing';
    if (changePercent < -5) return 'decreasing';
    return 'stable';
  }
  
  /**
   * Calculate overall system health
   */
  private calculateSystemHealth(): PerformanceMetrics['systemHealth'] {
    const metrics = this.metrics;
    let score = 100;
    
    // FPS impact
    if (metrics.fps < this.thresholds.fps.critical) score -= 30;
    else if (metrics.fps < this.thresholds.fps.warning) score -= 15;
    
    // Memory impact
    if (metrics.totalMemoryUsage > this.thresholds.memoryUsage.critical) score -= 25;
    else if (metrics.totalMemoryUsage > this.thresholds.memoryUsage.warning) score -= 10;
    
    // CPU impact
    if (metrics.cpuUsage > this.thresholds.cpuUsage.critical) score -= 20;
    else if (metrics.cpuUsage > this.thresholds.cpuUsage.warning) score -= 10;
    
    // Network impact
    if (metrics.networkLatency > this.thresholds.networkLatency.critical) score -= 15;
    else if (metrics.networkLatency > this.thresholds.networkLatency.warning) score -= 5;
    
    if (score >= 90) return 'excellent';
    if (score >= 75) return 'good';
    if (score >= 60) return 'fair';
    if (score >= 40) return 'poor';
    return 'critical';
  }
  
  /**
   * Analyze performance and trigger optimizations
   */
  private async analyzePerformance(): Promise<void> {
    const metrics = this.metrics;
    
    // Check for performance issues and report errors
    if (metrics.fps < this.thresholds.fps.critical) {
      handlePerformanceError(
        ERROR_CODES.PERFORMANCE_LOW_FPS,
        `Critical low frame rate: ${metrics.fps.toFixed(1)} FPS`,
        'fps',
        this.thresholds.fps.critical,
        metrics.fps,
        { trend: 'decreasing' }
      );
    }
    
    if (metrics.totalMemoryUsage > this.thresholds.memoryUsage.critical) {
      handlePerformanceError(
        ERROR_CODES.PERFORMANCE_MEMORY_LEAK,
        `Critical memory usage: ${(metrics.totalMemoryUsage / 1024 / 1024).toFixed(1)} MB`,
        'memory',
        this.thresholds.memoryUsage.critical,
        metrics.totalMemoryUsage,
        { trend: metrics.memoryTrend }
      );
    }
    
    // Trigger automatic optimizations
    if (metrics.systemHealth === 'poor' || metrics.systemHealth === 'critical') {
      await this.optimizePerformance();
    }
  }
  
  /**
   * Notify all subscribers
   */
  private notifySubscribers(): void {
    this.subscribers.forEach(callback => {
      try {
        callback(this.metrics);
      } catch (error) {
        console.error('[PerformanceMonitor] Error in subscriber callback:', error);
      }
    });
  }
  
  /**
   * Setup default performance optimizations
   */
  private setupOptimizations(): void {
    // Memory cleanup optimization
    this.registerOptimization({
      id: 'memory-cleanup',
      name: 'Memory Cleanup',
      description: 'Force garbage collection and clear caches',
      severity: 'medium',
      enabled: true,
      conditions: (metrics) => metrics.totalMemoryUsage > this.thresholds.memoryUsage.warning,
      action: async () => {
        try {
          // Force garbage collection if available
          if ('gc' in window) {
            (window as any).gc();
          }
          
          // Clear various caches
          if ('caches' in window) {
            const cacheNames = await caches.keys();
            await Promise.all(cacheNames.map(name => caches.delete(name)));
          }
          
          console.log('[PerformanceMonitor] Memory cleanup completed');
          return true;
        } catch (error) {
          console.error('[PerformanceMonitor] Memory cleanup failed:', error);
          return false;
        }
      }
    });
    
    // Reduce rendering quality
    this.registerOptimization({
      id: 'reduce-quality',
      name: 'Reduce Rendering Quality',
      description: 'Lower rendering quality to improve performance',
      severity: 'high',
      enabled: true,
      conditions: (metrics) => metrics.fps < this.thresholds.fps.warning,
      action: async () => {
        // This would interface with the rendering system
        console.log('[PerformanceMonitor] Reducing rendering quality');
        return true;
      }
    });
  }
  
  /**
   * Create initial metrics object
   */
  private createInitialMetrics(): PerformanceMetrics {
    return {
      fps: 60,
      frameTime: 16,
      renderLatency: 0,
      droppedFrames: 0,
      jsHeapUsed: 0,
      jsHeapTotal: 0,
      wasmMemoryUsed: 0,
      totalMemoryUsage: 0,
      memoryTrend: 'stable',
      dataProcessingTime: 0,
      dataTransferTime: 0,
      cacheHitRate: 0,
      networkLatency: 0,
      bandwidth: 0,
      packetLoss: 0,
      cpuUsage: 0,
      mainThreadBlockTime: 0,
      webgpuMemoryUsage: 0,
      gpuUtilization: 0,
      shaderCompileTime: 0,
      timestamp: Date.now(),
      systemHealth: 'good'
    };
  }
  
  /**
   * Cleanup resources
   */
  destroy(): void {
    this.stopMonitoring();
    this.subscribers = [];
    this.optimizations.clear();
    this.metricsHistory = [];
  }
}

// Global performance monitor instance
let globalPerformanceMonitor: PerformanceMonitor | null = null;

export function getGlobalPerformanceMonitor(): PerformanceMonitor {
  if (!globalPerformanceMonitor) {
    globalPerformanceMonitor = new PerformanceMonitor();
  }
  return globalPerformanceMonitor;
}

export function setGlobalPerformanceMonitor(monitor: PerformanceMonitor): void {
  globalPerformanceMonitor = monitor;
}