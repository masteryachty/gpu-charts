import { ComponentType, useEffect, useRef, forwardRef } from 'react';

interface PerformanceMetrics {
  componentName: string;
  renderTime: number;
  mountTime: number;
  updateCount: number;
  lastUpdate: number;
}

/**
 * Higher-order component that adds performance monitoring to any React component
 * 
 * @param Component - Component to wrap with performance monitoring
 * @param componentName - Name for tracking purposes
 * @returns Component with performance monitoring
 */
export function withPerformanceMonitoring<P extends Record<string, any>>(
  Component: ComponentType<P>,
  componentName: string
) {
  const PerformanceWrappedComponent = forwardRef<any, P>((props, ref) => {
    const mountTimeRef = useRef<number>();
    const renderCountRef = useRef(0);
    const lastRenderTimeRef = useRef<number>();

    useEffect(() => {
      // Track component mount
      const mountTime = performance.now();
      mountTimeRef.current = mountTime;

      // Log mount in development
      if (process.env.NODE_ENV === 'development') {
        console.log(`[Performance] ${componentName} mounted`);
      }

      return () => {
        // Track component unmount and total lifecycle
        const unmountTime = performance.now();
        const totalLifetime = mountTimeRef.current ? unmountTime - mountTimeRef.current : 0;
        
        const metrics: PerformanceMetrics = {
          componentName,
          renderTime: lastRenderTimeRef.current || 0,
          mountTime: mountTimeRef.current || 0,
          updateCount: renderCountRef.current,
          lastUpdate: unmountTime
        };

        // Log in development
        if (process.env.NODE_ENV === 'development') {
          console.log(`[Performance] ${componentName} unmounted after ${totalLifetime.toFixed(2)}ms`, metrics);
        }

        // Send to analytics in production
        if (typeof window !== 'undefined' && (window as any).analytics) {
          (window as any).analytics.track('component_lifecycle', {
            ...metrics,
            total_lifetime_ms: totalLifetime
          });
        }
      };
    }, []);

    useEffect(() => {
      // Track each render
      renderCountRef.current += 1;
      lastRenderTimeRef.current = performance.now();
    });

    return <Component ref={ref} {...props} />;
  });

  PerformanceWrappedComponent.displayName = `withPerformanceMonitoring(${componentName})`;
  
  return PerformanceWrappedComponent;
}

/**
 * Hook for tracking performance within functional components
 */
export function usePerformanceTracking(componentName: string, dependencies?: React.DependencyList) {
  const renderTimeRef = useRef<number>();
  const renderCountRef = useRef(0);

  useEffect(() => {
    const renderStart = performance.now();
    renderTimeRef.current = renderStart;
    
    return () => {
      const renderEnd = performance.now();
      const renderDuration = renderEnd - renderStart;
      renderCountRef.current += 1;

      if (process.env.NODE_ENV === 'development') {
        console.log(`[Performance] ${componentName} render ${renderCountRef.current} took ${renderDuration.toFixed(2)}ms`);
      }
    };
  }, dependencies);

  return {
    renderCount: renderCountRef.current,
    lastRenderTime: renderTimeRef.current
  };
}

/**
 * Performance observer for Core Web Vitals
 */
export class WebVitalsMonitor {
  private static instance: WebVitalsMonitor;
  private observers: PerformanceObserver[] = [];

  private constructor() {
    this.initializeObservers();
  }

  static getInstance(): WebVitalsMonitor {
    if (!WebVitalsMonitor.instance) {
      WebVitalsMonitor.instance = new WebVitalsMonitor();
    }
    return WebVitalsMonitor.instance;
  }

  private initializeObservers() {
    if (typeof window === 'undefined' || !window.PerformanceObserver) {
      return;
    }

    try {
      // Largest Contentful Paint (LCP)
      const lcpObserver = new PerformanceObserver((list) => {
        const entries = list.getEntries();
        const lastEntry = entries[entries.length - 1];
        this.reportMetric('LCP', lastEntry.startTime);
      });
      lcpObserver.observe({ entryTypes: ['largest-contentful-paint'] });
      this.observers.push(lcpObserver);

      // First Input Delay (FID)
      const fidObserver = new PerformanceObserver((list) => {
        const entries = list.getEntries();
        entries.forEach((entry: any) => {
          this.reportMetric('FID', entry.processingStart - entry.startTime);
        });
      });
      fidObserver.observe({ entryTypes: ['first-input'] });
      this.observers.push(fidObserver);

      // Cumulative Layout Shift (CLS)
      let clsValue = 0;
      const clsObserver = new PerformanceObserver((list) => {
        const entries = list.getEntries();
        entries.forEach((entry: any) => {
          if (!entry.hadRecentInput) {
            clsValue += entry.value;
            this.reportMetric('CLS', clsValue);
          }
        });
      });
      clsObserver.observe({ entryTypes: ['layout-shift'] });
      this.observers.push(clsObserver);

    } catch (error) {
      console.warn('Failed to initialize performance observers:', error);
    }
  }

  private reportMetric(name: string, value: number) {
    if (process.env.NODE_ENV === 'development') {
      console.log(`[WebVitals] ${name}: ${value.toFixed(2)}`);
    }

    // Send to analytics
    if (typeof window !== 'undefined' && (window as any).analytics) {
      (window as any).analytics.track('web_vital', {
        metric_name: name,
        value: value,
        url: window.location.href,
        timestamp: Date.now()
      });
    }
  }

  public disconnect() {
    this.observers.forEach(observer => observer.disconnect());
    this.observers = [];
  }
}

/**
 * Hook to initialize Web Vitals monitoring
 */
export function useWebVitalsMonitoring() {
  useEffect(() => {
    const monitor = WebVitalsMonitor.getInstance();
    
    return () => {
      monitor.disconnect();
    };
  }, []);
}

/**
 * Bundle size tracking utility
 */
export function trackBundleMetrics() {
  if (typeof window === 'undefined') return;

  // Track bundle size using Navigation API
  if ('performance' in window && 'getEntriesByType' in window.performance) {
    const navigationEntries = window.performance.getEntriesByType('navigation') as PerformanceNavigationTiming[];
    
    if (navigationEntries.length > 0) {
      const entry = navigationEntries[0];
      
      const metrics = {
        dns_time: entry.domainLookupEnd - entry.domainLookupStart,
        connection_time: entry.connectEnd - entry.connectStart,
        request_time: entry.responseStart - entry.requestStart,
        response_time: entry.responseEnd - entry.responseStart,
        dom_processing_time: entry.domContentLoadedEventStart - entry.responseEnd,
        load_event_time: entry.loadEventEnd - entry.loadEventStart,
        total_time: entry.loadEventEnd - entry.navigationStart
      };

      if (process.env.NODE_ENV === 'development') {
        console.log('[Bundle Metrics]', metrics);
      }

      if ((window as any).analytics) {
        (window as any).analytics.track('bundle_performance', metrics);
      }
    }
  }
}