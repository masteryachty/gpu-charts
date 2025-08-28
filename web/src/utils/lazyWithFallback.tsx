import { lazy, Suspense, ComponentType } from 'react';
import { ErrorBoundary } from '../components/error';
import { LoadingSkeleton } from '../components/loading/LoadingSkeleton';

interface LazyWrapperProps {
  fallback?: React.ReactNode;
  errorBoundaryName?: string;
  enableErrorRecovery?: boolean;
}

/**
 * Higher-order component for lazy loading with error boundaries and loading states
 * 
 * @param importFn - Dynamic import function
 * @param options - Configuration options for loading and error handling
 * @returns Lazy component with error boundary and loading fallback
 */
export function lazyWithFallback<T extends ComponentType<any>>(
  importFn: () => Promise<{ default: T }>,
  options: LazyWrapperProps = {}
) {
  const {
    fallback = <LoadingSkeleton height="100%" width="100%" />,
    errorBoundaryName = 'LazyComponent',
    enableErrorRecovery = true
  } = options;

  const LazyComponent = lazy(importFn);

  return function LazyWrapper(props: React.ComponentProps<T>) {
    return (
      <ErrorBoundary
        componentName={errorBoundaryName}
        enableAutoRecovery={enableErrorRecovery}
        enableReporting={true}
      >
        <Suspense fallback={fallback}>
          <LazyComponent {...props} />
        </Suspense>
      </ErrorBoundary>
    );
  };
}

/**
 * Preload a lazy component for better performance
 * 
 * @param importFn - Same import function used in lazyWithFallback
 */
export function preloadComponent(importFn: () => Promise<{ default: ComponentType<any> }>) {
  // Start loading the component but don't wait for it
  importFn().catch(error => {
    console.warn('Failed to preload component:', error);
  });
}

/**
 * Create a lazy component with specific loading skeleton for heavy chart components
 */
export function lazyChartComponent<T extends ComponentType<any>>(
  importFn: () => Promise<{ default: T }>,
  componentName: string
) {
  return lazyWithFallback(importFn, {
    fallback: (
      <div className="w-full h-full flex items-center justify-center bg-gray-900">
        <div className="text-center">
          <div className="animate-spin text-blue-500 text-3xl mb-4">⚡</div>
          <div className="text-white font-medium mb-2">Loading {componentName}</div>
          <div className="text-gray-400 text-sm">Initializing GPU acceleration...</div>
          <LoadingSkeleton height="2px" className="w-64 mx-auto mt-4" />
        </div>
      </div>
    ),
    errorBoundaryName: componentName,
    enableErrorRecovery: true
  });
}

/**
 * Create a lazy component for UI components with minimal loading state
 */
export function lazyUIComponent<T extends ComponentType<any>>(
  importFn: () => Promise<{ default: T }>,
  componentName: string,
  skeletonHeight: string = '3rem'
) {
  return lazyWithFallback(importFn, {
    fallback: <LoadingSkeleton height={skeletonHeight} className="w-full" />,
    errorBoundaryName: componentName,
    enableErrorRecovery: true
  });
}

/**
 * Utility to check if component lazy loading is supported
 */
export const isLazyLoadingSupported = () => {
  return 'loading' in HTMLImageElement.prototype;
};

/**
 * Performance monitoring for lazy loading
 */
export function trackComponentLoad(componentName: string, startTime: number) {
  const loadTime = performance.now() - startTime;
  
  // Log for development
  if (process.env.NODE_ENV === 'development') {
    console.log(`[LazyLoad] ${componentName} loaded in ${loadTime.toFixed(2)}ms`);
  }
  
  // Track in production analytics if available
  if (typeof window !== 'undefined' && (window as any).analytics) {
    (window as any).analytics.track('component_lazy_loaded', {
      component_name: componentName,
      load_time_ms: loadTime,
      browser: navigator.userAgent
    });
  }
}