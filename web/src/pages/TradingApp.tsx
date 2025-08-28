import { useState, useEffect, lazy, Suspense } from 'react';
import { Routes, Route } from 'react-router-dom';
import { useAppStore } from '../store/useAppStore';
import { ErrorBoundary, DataErrorBoundary, UIErrorBoundary } from '../components/error';
import { AppLoadingScreen } from '../components/loading/AppLoadingScreen';
import { LoadingSkeleton, ChartLoadingSkeleton } from '../components/loading/LoadingSkeleton';
import { useWasmInitialization } from '../hooks/useWasmInitialization';
import { Chart } from '@pkg/wasm_bridge.js';

// Lazy load heavy components with specific loading skeletons
const Header = lazy(() => import('../components/layout/Header'));
const Sidebar = lazy(() => import('../components/layout/Sidebar'));
const StatusBar = lazy(() => import('../components/layout/StatusBar'));
const WasmCanvas = lazy(() => import('../components/chart/WasmCanvas'));
const ChartControls = lazy(() => import('../components/chart/ChartControls'));
const ChartLegend = lazy(() => import('../components/chart/ChartLegend'));

// Development-only components
const DevPerformanceDashboard = lazy(() => import('../components/debug/PerformanceDashboard').then(module => ({
  default: module.DevPerformanceDashboard
})));
// import DataFetchingMonitor from '../components/monitoring/DataFetchingMonitor'; // Disabled temporarily

function ChartView() {
  const [chartInstance, setChartInstance] = useState<Chart | undefined>(undefined);
  const [appliedPreset, setAppliedPreset] = useState<string | undefined>(undefined);

  // Get store state and actions
  const { symbol, preset, startTime, endTime, setCurrentSymbol, setTimeRange, setPreset, setBaseSymbol, comparisonMode, selectedExchanges } = useAppStore();
  const [activePreset, setActivePreset] = useState<string | undefined>(preset);


  useEffect(() => {
    if (preset && chartInstance) {
      // Check if we're in comparison mode with multiple exchanges
      if (comparisonMode && selectedExchanges && selectedExchanges.length > 0) {
        console.log('[TradingApp] Applying preset with multiple symbols:', preset, selectedExchanges);
        
        // Check if apply_preset_and_symbols exists (new method)
        if ('apply_preset_and_symbols' in chartInstance) {
          const symbolsArray = selectedExchanges;
          (chartInstance as any).apply_preset_and_symbols(preset, symbolsArray)
            .then(() => {
              console.log('[TradingApp] Successfully applied preset with multiple symbols');
              setAppliedPreset(preset);
            })
            .catch((error: Error) => {
              console.error('[TradingApp] Failed to apply preset with multiple symbols:', error);
            });
        } else {
          // Fallback to single symbol
          if (symbol) {
            chartInstance.apply_preset_and_symbol(preset, symbol)
              .then(() => {
                console.log('[TradingApp] Successfully applied preset and symbol');
                setAppliedPreset(preset);
              })
              .catch((error: Error) => {
                console.error('[TradingApp] Failed to apply preset:', error);
              });
          }
        }
      } else if (symbol) {
        // Single symbol mode
        const presetStartTime = performance.now();
        console.log('[PERF] TradingApp applying preset and symbol:', preset, symbol);
        chartInstance.apply_preset_and_symbol(preset, symbol)
          .then(() => {
            console.log(`[PERF] TradingApp preset applied successfully in ${(performance.now() - presetStartTime).toFixed(2)}ms`);
            console.log('[TradingApp] Successfully applied preset and symbol');
            setAppliedPreset(preset);
          })
          .catch((error: Error) => {
            console.error('[TradingApp] Failed to apply preset:', error);
          });
      }
    }
    setActivePreset(preset);
  }, [chartInstance, preset, symbol, comparisonMode, selectedExchanges]);

  // Sync activePreset with store's metricPreset
  useEffect(() => {
    setActivePreset(preset);
  }, [preset]);

  // Parse URL parameters and update store
  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search);

    // Parse topic (symbol)
    const topic = urlParams.get('topic');
    if (topic) {
      setCurrentSymbol(topic);
      // Extract base symbol from the topic (e.g., "coinbase:BTC-USD" -> "BTC-USD")
      const baseSymbol = topic.includes(':') ? topic.split(':')[1] : topic;
      setBaseSymbol(baseSymbol);
    }

    // Parse start and end timestamps
    const startParam = urlParams.get('start');
    const endParam = urlParams.get('end');

    if (startParam) {
      const startTime = parseInt(startParam, 10);
      const endTime = endParam ? parseInt(endParam, 10) : Math.ceil((new Date()).valueOf() / 1e3);


      // Validate timestamps
      if (!isNaN(startTime) && !isNaN(endTime) && startTime < endTime) {
        setTimeRange(startTime, endTime);
      } else {
      }
    }
  }, [setCurrentSymbol, setTimeRange, setBaseSymbol]); // Include dependencies
  
  // Note: Time range updates are now handled in WasmCanvas component
  // which watches for startTime/endTime changes and calls update_time_range

  return (
    <div className="flex-1 flex">
      <UIErrorBoundary componentName="Sidebar">
        <aside id="sidebar" data-skip-target="sidebar" role="navigation" aria-label="Main navigation">
          <Suspense fallback={
            <div className="w-64 bg-gray-800 border-r border-gray-700 p-4">
              <LoadingSkeleton height="2rem" className="mb-4" />
              <LoadingSkeleton height="1rem" className="mb-6 w-3/4" />
              {Array.from({ length: 6 }).map((_, i) => (
                <LoadingSkeleton key={i} height="3rem" className="mb-3" />
              ))}
            </div>
          }>
            <Sidebar />
          </Suspense>
        </aside>
      </UIErrorBoundary>
      <main id="main-content" data-skip-target="main-content" className="flex-1 flex flex-col" role="main">
        <div className="flex-1 p-6">
          <div className="h-full flex flex-col">
            <header className="mb-6 flex items-center justify-between">
              <div>
                <h1 className="text-3xl font-bold text-white mb-2">Trading Dashboard</h1>
                <p className="text-gray-400">
                  Real-time store synchronization with WebGPU acceleration
                  {activePreset && (
                    <span className="ml-2 px-2 py-1 bg-blue-600 text-white text-xs rounded">
                      Preset: {activePreset}
                    </span>
                  )}
                </p>
              </div>
            </header>

            <div className="flex-1 flex gap-6 flex-col lg:flex-row">
              {/* Chart Controls Panel */}
              <section 
                id="chart-controls" 
                data-skip-target="chart-controls" 
                className="w-full lg:w-80 flex-shrink-0 space-y-4"
                role="region"
                aria-label="Chart controls"
              >
                <DataErrorBoundary dataSource="chart-controls">
                  <Suspense fallback={
                    <div className="space-y-4">
                      <LoadingSkeleton height="2rem" className="mb-2" />
                      <LoadingSkeleton height="1rem" className="mb-4 w-2/3" />
                      <div className="grid grid-cols-2 gap-2 mb-4">
                        {Array.from({ length: 4 }).map((_, i) => (
                          <LoadingSkeleton key={i} height="2.5rem" />
                        ))}
                      </div>
                      <LoadingSkeleton height="8rem" />
                      <LoadingSkeleton height="6rem" />
                    </div>
                  }>
                    <ChartControls
                      chartInstance={chartInstance}
                      appliedPreset={appliedPreset}
                      onPresetChange={(preset) => {
                        setActivePreset(preset);
                        setPreset(preset);
                      }}
                    />
                  </Suspense>
                </DataErrorBoundary>
              </section>

              {/* Main Chart Area */}
              <section 
                id="chart" 
                data-skip-target="chart" 
                className="flex-1 flex flex-col"
                role="region"
                aria-label="Financial chart"
              >
                {/* Chart Legend - positioned over the chart */}
                <div className="relative">
                  <DataErrorBoundary dataSource="wasm-canvas">
                    <Suspense fallback={<ChartLoadingSkeleton className="w-full h-full" />}>
                      <WasmCanvas
                        onChartReady={setChartInstance}
                      />
                    </Suspense>
                  </DataErrorBoundary>
                  <div className="absolute top-4 right-4 z-10">
                    <UIErrorBoundary componentName="ChartLegend">
                      <Suspense fallback={
                        <div className="bg-gray-800/90 backdrop-blur-sm border border-gray-600 rounded-lg p-4">
                          <LoadingSkeleton height="1rem" className="mb-2 w-24" />
                          <LoadingSkeleton height="0.75rem" className="w-32" />
                        </div>
                      }>
                        <ChartLegend />
                      </Suspense>
                    </UIErrorBoundary>
                  </div>
                </div>
              </section>
            </div>
          </div>
        </div>

        <UIErrorBoundary componentName="StatusBar">
          <footer role="contentinfo" aria-label="Status information">
            <Suspense fallback={
              <div className="h-8 bg-gray-800 border-t border-gray-700 flex items-center px-4">
                <LoadingSkeleton height="1rem" className="w-32" />
                <div className="flex-1" />
                <LoadingSkeleton height="1rem" className="w-24" />
              </div>
            }>
              <StatusBar />
            </Suspense>
          </footer>
        </UIErrorBoundary>
      </main>
    </div>
  );
}

export default function TradingApp() {
  const { isInitialized } = useWasmInitialization();
  const [showLoadingScreen, setShowLoadingScreen] = useState(true);

  const handleTopLevelError = (error: Error, errorInfo: React.ErrorInfo) => {
    // Top-level error tracking
    console.error('[TradingApp] Top-level application error:', error);
    
    // Report to external service in production
    if (process.env.NODE_ENV === 'production') {
      // Example: Sentry.captureException(error, { contexts: { react: { componentStack: errorInfo.componentStack } } });
      console.warn('In production, this error would be reported to error tracking service');
    }
  };

  const handleInitialized = () => {
    // Small delay for smooth transition
    setTimeout(() => {
      setShowLoadingScreen(false);
    }, 500);
  };

  // Show loading screen until WASM is initialized
  if (showLoadingScreen && !isInitialized) {
    return <AppLoadingScreen onInitialized={handleInitialized} />;
  }

  return (
    <ErrorBoundary
      componentName="TradingApp"
      onError={handleTopLevelError}
      enableReporting={true}
      enableAutoRecovery={false} // Top-level shouldn't auto-retry
    >
      <div className="h-screen bg-gray-900 flex flex-col">
        <UIErrorBoundary componentName="Header">
          <header id="header" data-skip-target="header" role="banner">
            <Suspense fallback={
              <div className="h-16 bg-gray-800 border-b border-gray-700 flex items-center justify-between px-6">
                <LoadingSkeleton height="2rem" className="w-32" />
                <div className="flex items-center gap-4">
                  <LoadingSkeleton height="2rem" className="w-24" />
                  <LoadingSkeleton height="2rem" className="w-16 rounded-full" />
                </div>
              </div>
            }>
              <Header />
            </Suspense>
          </header>
        </UIErrorBoundary>
        
        <ErrorBoundary componentName="Router">
          <Routes>
            <Route path="/" element={<ChartView />} />
            <Route path="/chart/:symbol" element={<ChartView />} />
          </Routes>
        </ErrorBoundary>

        {/* Development Performance Dashboard */}
        {process.env.NODE_ENV === 'development' && (
          <Suspense fallback={null}>
            <DevPerformanceDashboard />
          </Suspense>
        )}
      </div>
    </ErrorBoundary>
  );
}