/**
 * Comprehensive Performance Monitoring System for GPU Charts
 * 
 * Tracks application performance, WebGPU metrics, bundle loading,
 * and user interactions for optimization insights.
 */

interface PerformanceMetric {
  name: string;
  value: number;
  unit: string;
  timestamp: number;
  category: 'rendering' | 'loading' | 'interaction' | 'memory' | 'network';
  metadata?: Record<string, any>;
}

interface ChartPerformanceMetrics {
  fps: number;
  renderTime: number;
  dataPoints: number;
  gpuMemoryUsage?: number;
  wasmHeapSize?: number;
}

class PerformanceMonitor {
  private static instance: PerformanceMonitor;
  private metrics: PerformanceMetric[] = [];
  private observers: PerformanceObserver[] = [];
  private isRecording = false;
  private maxMetrics = 1000; // Prevent memory leaks

  private constructor() {
    this.initializeObservers();
    this.startRecording();
  }

  static getInstance(): PerformanceMonitor {
    if (!PerformanceMonitor.instance) {
      PerformanceMonitor.instance = new PerformanceMonitor();
    }
    return PerformanceMonitor.instance;
  }

  private initializeObservers() {
    if (typeof window === 'undefined' || !window.PerformanceObserver) {
      return;
    }

    try {
      // Navigation timing
      const navigationObserver = new PerformanceObserver((list) => {
        const entries = list.getEntries();
        entries.forEach((entry: any) => {
          this.recordMetric({
            name: 'page_load_time',
            value: entry.loadEventEnd - entry.navigationStart,
            unit: 'ms',
            timestamp: Date.now(),
            category: 'loading',
            metadata: {
              dns_time: entry.domainLookupEnd - entry.domainLookupStart,
              connection_time: entry.connectEnd - entry.connectStart,
              response_time: entry.responseEnd - entry.responseStart,
              dom_ready: entry.domContentLoadedEventEnd - entry.navigationStart
            }
          });
        });
      });
      navigationObserver.observe({ entryTypes: ['navigation'] });
      this.observers.push(navigationObserver);

      // Resource timing for bundle analysis
      const resourceObserver = new PerformanceObserver((list) => {
        const entries = list.getEntries();
        entries.forEach((entry: any) => {
          if (entry.name.includes('.js') || entry.name.includes('.wasm') || entry.name.includes('.css')) {
            this.recordMetric({
              name: 'resource_load_time',
              value: entry.responseEnd - entry.startTime,
              unit: 'ms',
              timestamp: Date.now(),
              category: 'loading',
              metadata: {
                resource_name: entry.name,
                resource_type: entry.initiatorType,
                size: entry.transferSize,
                cache_hit: entry.transferSize === 0 && entry.encodedBodySize > 0
              }
            });
          }
        });
      });
      resourceObserver.observe({ entryTypes: ['resource'] });
      this.observers.push(resourceObserver);

      // Memory usage (if available)
      if ('memory' in performance) {
        setInterval(() => {
          const memory = (performance as any).memory;
          this.recordMetric({
            name: 'memory_usage',
            value: memory.usedJSHeapSize,
            unit: 'bytes',
            timestamp: Date.now(),
            category: 'memory',
            metadata: {
              total_heap: memory.totalJSHeapSize,
              heap_limit: memory.jsHeapSizeLimit,
              usage_percentage: (memory.usedJSHeapSize / memory.jsHeapSizeLimit) * 100
            }
          });
        }, 10000); // Every 10 seconds
      }

      // Long tasks detection
      const longTaskObserver = new PerformanceObserver((list) => {
        const entries = list.getEntries();
        entries.forEach((entry: any) => {
          this.recordMetric({
            name: 'long_task',
            value: entry.duration,
            unit: 'ms',
            timestamp: Date.now(),
            category: 'rendering',
            metadata: {
              attribution: entry.attribution?.map((attr: any) => ({
                name: attr.name,
                container_type: attr.containerType,
                container_src: attr.containerSrc
              }))
            }
          });
        });
      });
      longTaskObserver.observe({ entryTypes: ['longtask'] });
      this.observers.push(longTaskObserver);

    } catch (error) {
      console.warn('Failed to initialize some performance observers:', error);
    }
  }

  private recordMetric(metric: PerformanceMetric) {
    if (!this.isRecording) return;

    this.metrics.push(metric);

    // Prevent memory leaks
    if (this.metrics.length > this.maxMetrics) {
      this.metrics = this.metrics.slice(-this.maxMetrics / 2);
    }

    // Log in development
    if (process.env.NODE_ENV === 'development') {
      console.log(`[PerformanceMonitor] ${metric.name}: ${metric.value.toFixed(2)}${metric.unit}`, metric.metadata);
    }

    // Send to analytics in production
    if (process.env.NODE_ENV === 'production' && (window as any).analytics) {
      (window as any).analytics.track('performance_metric', {
        metric_name: metric.name,
        value: metric.value,
        unit: metric.unit,
        category: metric.category,
        timestamp: metric.timestamp,
        url: window.location.href,
        user_agent: navigator.userAgent,
        ...metric.metadata
      });
    }
  }

  /**
   * Track chart-specific performance metrics
   */
  public trackChartPerformance(metrics: ChartPerformanceMetrics) {
    this.recordMetric({
      name: 'chart_fps',
      value: metrics.fps,
      unit: 'fps',
      timestamp: Date.now(),
      category: 'rendering',
      metadata: {
        render_time: metrics.renderTime,
        data_points: metrics.dataPoints,
        gpu_memory: metrics.gpuMemoryUsage,
        wasm_heap: metrics.wasmHeapSize
      }
    });

    this.recordMetric({
      name: 'chart_render_time',
      value: metrics.renderTime,
      unit: 'ms',
      timestamp: Date.now(),
      category: 'rendering',
      metadata: {
        fps: metrics.fps,
        data_points: metrics.dataPoints
      }
    });

    // Alert if performance is degrading
    if (metrics.fps < 30) {
      console.warn(`[PerformanceMonitor] Low FPS detected: ${metrics.fps}fps with ${metrics.dataPoints} data points`);
    }
  }

  /**
   * Track user interactions
   */
  public trackInteraction(action: string, duration: number, metadata?: Record<string, any>) {
    this.recordMetric({
      name: 'user_interaction',
      value: duration,
      unit: 'ms',
      timestamp: Date.now(),
      category: 'interaction',
      metadata: {
        action,
        ...metadata
      }
    });
  }

  /**
   * Track WASM loading performance
   */
  public trackWasmLoad(loadTime: number, wasmSize?: number) {
    this.recordMetric({
      name: 'wasm_load_time',
      value: loadTime,
      unit: 'ms',
      timestamp: Date.now(),
      category: 'loading',
      metadata: {
        wasm_size: wasmSize,
        browser: navigator.userAgent,
        webgpu_supported: 'gpu' in navigator
      }
    });
  }

  /**
   * Track WebGPU initialization
   */
  public trackWebGPUInit(initTime: number, success: boolean, error?: string) {
    this.recordMetric({
      name: 'webgpu_init_time',
      value: initTime,
      unit: 'ms',
      timestamp: Date.now(),
      category: 'loading',
      metadata: {
        success,
        error,
        gpu_info: success ? 'Available' : 'Not supported',
        browser: navigator.userAgent
      }
    });
  }

  /**
   * Get performance summary
   */
  public getPerformanceSummary() {
    const now = Date.now();
    const recentMetrics = this.metrics.filter(m => now - m.timestamp < 60000); // Last minute

    const summary = {
      total_metrics: this.metrics.length,
      recent_metrics: recentMetrics.length,
      categories: this.metrics.reduce((acc, metric) => {
        acc[metric.category] = (acc[metric.category] || 0) + 1;
        return acc;
      }, {} as Record<string, number>),
      avg_chart_fps: this.getAverageMetric('chart_fps', 30000), // Last 30 seconds
      avg_render_time: this.getAverageMetric('chart_render_time', 30000),
      long_tasks: this.metrics.filter(m => m.name === 'long_task' && now - m.timestamp < 60000).length,
      memory_usage: this.getLatestMetric('memory_usage')
    };

    return summary;
  }

  private getAverageMetric(name: string, timeWindow: number): number | null {
    const now = Date.now();
    const relevantMetrics = this.metrics.filter(m => 
      m.name === name && now - m.timestamp < timeWindow
    );

    if (relevantMetrics.length === 0) return null;

    const sum = relevantMetrics.reduce((acc, m) => acc + m.value, 0);
    return sum / relevantMetrics.length;
  }

  private getLatestMetric(name: string): PerformanceMetric | null {
    for (let i = this.metrics.length - 1; i >= 0; i--) {
      if (this.metrics[i].name === name) {
        return this.metrics[i];
      }
    }
    return null;
  }

  /**
   * Export metrics for analysis
   */
  public exportMetrics() {
    return {
      metrics: this.metrics,
      summary: this.getPerformanceSummary(),
      browser_info: {
        user_agent: navigator.userAgent,
        language: navigator.language,
        platform: navigator.platform,
        connection: (navigator as any).connection?.effectiveType,
        memory: (navigator as any).deviceMemory,
        cpu_cores: navigator.hardwareConcurrency
      },
      timestamp: Date.now()
    };
  }

  /**
   * Start recording metrics
   */
  public startRecording() {
    this.isRecording = true;
  }

  /**
   * Stop recording metrics
   */
  public stopRecording() {
    this.isRecording = false;
  }

  /**
   * Clear all metrics
   */
  public clearMetrics() {
    this.metrics = [];
  }

  /**
   * Disconnect all observers
   */
  public disconnect() {
    this.observers.forEach(observer => observer.disconnect());
    this.observers = [];
    this.isRecording = false;
  }
}

// Singleton instance
export const performanceMonitor = PerformanceMonitor.getInstance();

// React hook for easy integration
export function usePerformanceMonitoring() {
  const monitor = PerformanceMonitor.getInstance();
  
  return {
    trackChartPerformance: monitor.trackChartPerformance.bind(monitor),
    trackInteraction: monitor.trackInteraction.bind(monitor),
    trackWasmLoad: monitor.trackWasmLoad.bind(monitor),
    trackWebGPUInit: monitor.trackWebGPUInit.bind(monitor),
    getPerformanceSummary: monitor.getPerformanceSummary.bind(monitor),
    exportMetrics: monitor.exportMetrics.bind(monitor)
  };
}

// Initialize performance monitoring on import
if (typeof window !== 'undefined') {
  // Track initial page load metrics
  window.addEventListener('load', () => {
    setTimeout(() => {
      performanceMonitor.trackInteraction('page_load', performance.now(), {
        referrer: document.referrer,
        page: window.location.pathname
      });
    }, 100);
  });

  // Track page visibility changes
  document.addEventListener('visibilitychange', () => {
    performanceMonitor.trackInteraction('visibility_change', performance.now(), {
      visible: !document.hidden
    });
  });

  // Global error tracking
  window.addEventListener('error', (event) => {
    console.error('[PerformanceMonitor] JavaScript Error:', event.error);
  });

  window.addEventListener('unhandledrejection', (event) => {
    console.error('[PerformanceMonitor] Unhandled Promise Rejection:', event.reason);
  });
}