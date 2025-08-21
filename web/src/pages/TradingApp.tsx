import { useState, useEffect } from 'react';
import { Routes, Route } from 'react-router-dom';
import { useAppStore } from '../store/useAppStore';
import Header from '../components/layout/Header';
import Sidebar from '../components/layout/Sidebar';
import StatusBar from '../components/layout/StatusBar';
import WasmCanvas from '../components/chart/WasmCanvas';
import ChartControls from '../components/chart/ChartControls';
import ChartLegend from '../components/chart/ChartLegend';
import { Chart } from '@pkg/wasm_bridge.js';
// import DataFetchingMonitor from '../components/monitoring/DataFetchingMonitor'; // Disabled temporarily

function ChartView() {
  const [chartInstance, setChartInstance] = useState<Chart | undefined>(undefined);
  const [appliedPreset, setAppliedPreset] = useState<string | undefined>(undefined);

  // Get store state and actions
  const { symbol, preset, startTime, endTime, setCurrentSymbol, setTimeRange, setPreset, setBaseSymbol } = useAppStore();
  const [activePreset, setActivePreset] = useState<string | undefined>(preset);


  useEffect(() => {
    if (preset && symbol && chartInstance) {
      // Apply preset and symbol - returns a promise
      chartInstance.apply_preset_and_symbol(preset, symbol)
        .then(() => {
          // Preset has been fully applied and data fetched
          setAppliedPreset(preset);
        })
        .catch((error: Error) => {
          console.error('[TradingApp] Failed to apply preset:', error);
        });
    }
    setActivePreset(preset);
  }, [chartInstance, preset, symbol]);

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
                  appliedPreset={appliedPreset}
                  onPresetChange={(preset) => {
                    setActivePreset(preset);
                    setPreset(preset);
                  }}
                />

              </div>

              {/* Main Chart Area */}
              <div className="flex-1 flex flex-col">
                {/* Chart Legend - positioned over the chart */}
                <div className="relative">
                  <WasmCanvas
                    onChartReady={setChartInstance}
                  />
                  <div className="absolute top-4 right-4 z-10">
                    <ChartLegend />
                  </div>
                </div>
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