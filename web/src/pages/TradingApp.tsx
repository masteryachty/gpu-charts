import { useState, useEffect } from 'react';
import { Routes, Route } from 'react-router-dom';
import { useAppStore } from '../store/useAppStore';
import Header from '../components/layout/Header';
import Sidebar from '../components/layout/Sidebar';
import StatusBar from '../components/layout/StatusBar';
import WasmCanvas from '../components/chart/WasmCanvas';
import ChartControls from '../components/chart/ChartControls';
// import DataFetchingMonitor from '../components/monitoring/DataFetchingMonitor'; // Disabled temporarily

function ChartView() {
  const [chartInstance, setChartInstance] = useState<any>(null);

  // Get store state and actions
  const { ChartStateConfig, setCurrentSymbol, setTimeRange, setMetricPreset } = useAppStore();
  const [activePreset, setActivePreset] = useState<string | null>(ChartStateConfig.metricPreset);

  // Sync activePreset with store's metricPreset
  useEffect(() => {
    setActivePreset(ChartStateConfig.metricPreset);
  }, [ChartStateConfig.metricPreset]);

  // Parse URL parameters and update store
  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search);

    // Parse topic (symbol)
    const topic = urlParams.get('topic');
    if (topic) {
      console.log('[TradingApp] Setting symbol from URL:', topic);
      setCurrentSymbol(topic);
    }

    // Parse start and end timestamps
    const startParam = urlParams.get('start');
    const endParam = urlParams.get('end');

    if (startParam) {
      const startTime = parseInt(startParam, 10);
      const endTime = endParam ? parseInt(endParam, 10) : Math.ceil((new Date()).valueOf() / 1e3);


      // Validate timestamps
      if (!isNaN(startTime) && !isNaN(endTime) && startTime < endTime) {
        console.log('[TradingApp] Setting time range from URL:', {
          start: startTime,
          end: endTime,
          startDate: new Date(startTime * 1000).toISOString(),
          endDate: new Date(endTime * 1000).toISOString()
        });
        setTimeRange(startTime, endTime);
      } else {
        console.warn('[TradingApp] Invalid timestamp parameters:', { startParam, endParam });
      }
    }
  }, [setCurrentSymbol, setTimeRange]); // Include dependencies

  // Check for debug mode in URL params
  const urlParams = new URLSearchParams(window.location.search);
  const debugFromUrl = urlParams.get('debug') === 'true';

  return (
    <div className="flex-1 flex">
      <Sidebar />

      <main className="flex-1 flex flex-col">
        <div className="flex-1 p-6">
          <div className="h-full flex flex-col">
            <div className="mb-6 flex items-center justify-between">
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

            </div>

            <div className="flex-1 flex gap-6 flex-col lg:flex-row">
              {/* Chart Controls Panel */}
              <div className="w-full lg:w-80 flex-shrink-0 space-y-4">
                <ChartControls
                  chartInstance={chartInstance}
                  onPresetChange={(preset) => {
                    setActivePreset(preset);
                    setMetricPreset(preset);
                  }}
                />

                {/* Data Fetching Monitor - Disabled temporarily
                <DataFetchingMonitor 
                  showDetailedInfo={debugMode}
                  enableManualControls={true}
                  showActivity={debugMode}
                  compactMode={!debugMode}
                />*/}
              </div>

              {/* Main Chart Area */}
              <div className="flex-1 flex flex-col">
                <WasmCanvas
                  enableAutoSync={true}
                  debounceMs={100}
                  onChartReady={setChartInstance}
                  activePreset={activePreset}
                />
              </div>
            </div>
          </div>
        </div>

        <StatusBar />
      </main>
    </div>
  );
}

export default function TradingApp() {
  return (
    <div className="h-screen bg-gray-900 flex flex-col">
      <Header />
      <Routes>
        <Route path="/" element={<ChartView />} />
        <Route path="/chart/:symbol" element={<ChartView />} />
      </Routes>
    </div>
  );
}