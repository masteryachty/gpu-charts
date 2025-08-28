import { Routes, Route } from 'react-router-dom';
import { lazy, Suspense } from 'react';
import * as React from 'react';
import { ErrorBoundary } from './index';
import { LoadingSkeleton } from './components/loading/LoadingSkeleton';

// Lazy load heavy components
const HomePage = lazy(() => import('./pages/HomePage'));
const TradingApp = lazy(() => import('./pages/TradingApp'));
const PricingPage = lazy(() => import('./pages/PricingPage'));
const AboutPage = lazy(() => import('./pages/AboutPage'));
const DocsPage = lazy(() => import('./pages/DocsPage'));
import { useAppStore } from './store/useAppStore';
import { KeyboardNavigationProvider } from './contexts/KeyboardNavigationContext';
import { LoadingProvider } from './contexts/LoadingContext';
import { TourProvider, FirstTimeUserDetector } from './components/onboarding/TourManager';
import { TourButton } from './components/onboarding/TourButton';
import { SkipLinks, KeyboardHelpButton } from './components/navigation/SkipLinks';
import { performanceMonitor, usePerformanceMonitoring } from './utils/performanceMonitor';
import { useWebVitalsMonitoring, trackBundleMetrics } from './utils/withPerformanceMonitoring';

// Initialize the integration system - temporarily disabled for testing
// initializeIntegrationSystem({
//   enableErrorReporting: true,
//   enablePerformanceMonitoring: true,
//   debugMode: process.env.NODE_ENV === 'development'
// });

function App() {
  // Make store available globally for testing
  const store = useAppStore;
  if (typeof window !== 'undefined') {
    (window as any).__zustandStore = store;
    // Initialize performance monitoring
    (window as any).__performanceMonitor = performanceMonitor;
  }

  // Initialize performance monitoring hooks
  useWebVitalsMonitoring();
  const { trackInteraction } = usePerformanceMonitoring();

  // Track app initialization
  React.useEffect(() => {
    const startTime = performance.now();
    trackBundleMetrics();
    
    trackInteraction('app_init', performance.now() - startTime, {
      route: window.location.pathname,
      referrer: document.referrer
    });

    // Performance summary logging in development
    if (process.env.NODE_ENV === 'development') {
      const logPerformanceSummary = () => {
        console.log('[App] Performance Summary:', performanceMonitor.getPerformanceSummary());
      };
      
      // Log summary every 30 seconds in development
      const interval = setInterval(logPerformanceSummary, 30000);
      return () => clearInterval(interval);
    }
  }, [trackInteraction]);

  return (
    <ErrorBoundary
      enableAutoRecovery={true}
      maxRetryAttempts={3}
      enableReporting={true}
      componentName="App"
    >
      <LoadingProvider>
        <TourProvider>
          <KeyboardNavigationProvider>
            <div className="min-h-screen bg-bg-primary">
              <SkipLinks />
              
              <Routes>
                <Route 
                  path="/" 
                  element={
                    <Suspense fallback={
                      <div className="min-h-screen bg-bg-primary flex items-center justify-center">
                        <div className="max-w-md w-full">
                          <LoadingSkeleton height="2rem" className="mb-4" />
                          <LoadingSkeleton height="1rem" className="mb-8 w-3/4" />
                          <LoadingSkeleton height="12rem" />
                        </div>
                      </div>
                    }>
                      <HomePage />
                    </Suspense>
                  } 
                />
                <Route 
                  path="/app/*" 
                  element={
                    <Suspense fallback={
                      <div className="h-screen bg-gray-900 flex items-center justify-center">
                        <div className="max-w-lg w-full p-8">
                          <div className="text-center mb-8">
                            <div className="animate-spin text-blue-500 text-4xl mb-4">⚡</div>
                            <h2 className="text-xl font-semibold text-white mb-2">Loading Trading Platform</h2>
                            <p className="text-gray-400">Initializing WebGPU and WASM modules...</p>
                          </div>
                          <LoadingSkeleton height="3rem" className="mb-4" />
                          <div className="grid grid-cols-3 gap-4 mb-6">
                            <LoadingSkeleton height="8rem" />
                            <LoadingSkeleton height="8rem" />
                            <LoadingSkeleton height="8rem" />
                          </div>
                          <LoadingSkeleton height="16rem" />
                        </div>
                      </div>
                    }>
                      <TradingApp />
                    </Suspense>
                  } 
                />
                <Route 
                  path="/pricing" 
                  element={
                    <Suspense fallback={<LoadingSkeleton height="100vh" />}>
                      <PricingPage />
                    </Suspense>
                  } 
                />
                <Route 
                  path="/about" 
                  element={
                    <Suspense fallback={<LoadingSkeleton height="100vh" />}>
                      <AboutPage />
                    </Suspense>
                  } 
                />
                <Route 
                  path="/docs" 
                  element={
                    <Suspense fallback={<LoadingSkeleton height="100vh" />}>
                      <DocsPage />
                    </Suspense>
                  } 
                />
              </Routes>
              
              <KeyboardHelpButton />
              <TourButton variant="floating" />
              <FirstTimeUserDetector />
              
              {/* Global error notification system - temporarily disabled for testing */}
              {/* <ErrorNotificationCenter
                position="top-right"
                maxNotifications={5}
                autoHideTimeoutMs={8000}
                enableSounds={false}
                showDetailedInfo={false}
              /> */}
            </div>
          </KeyboardNavigationProvider>
        </TourProvider>
      </LoadingProvider>
    </ErrorBoundary>
  );
}

export default App;