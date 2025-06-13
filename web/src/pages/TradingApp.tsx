import { useState, useEffect } from 'react';
import { Routes, Route } from 'react-router-dom';
import { useAppStore } from '../store/useAppStore';
import Header from '../components/layout/Header';
import Sidebar from '../components/layout/Sidebar';
import StatusBar from '../components/layout/StatusBar';
import WasmCanvas from '../components/chart/WasmCanvas';
import ChartControls from '../components/chart/ChartControls';
import DataFetchingMonitor from '../components/monitoring/DataFetchingMonitor';

function ChartView() {
  const [showDebugMode, setShowDebugMode] = useState(false);
  const [showSubscriptionInfo, setShowSubscriptionInfo] = useState(false);
  const [enableChangeTracking, setEnableChangeTracking] = useState(false);

  // Get store actions
  const { setCurrentSymbol, setTimeRange } = useAppStore();

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
    
    if (startParam && endParam) {
      const startTime = parseInt(startParam, 10);
      const endTime = parseInt(endParam, 10);
      
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
  }, []); // Run only once on mount

  // Check for debug mode in URL params
  const urlParams = new URLSearchParams(window.location.search);
  const debugFromUrl = urlParams.get('debug') === 'true';
  const debugMode = showDebugMode || debugFromUrl;

  return (
    <div className="flex-1 flex">
      <Sidebar />
      
      <main className="flex-1 flex flex-col">
        <div className="flex-1 p-6">
          <div className="h-full flex flex-col">
            <div className="mb-6 flex items-center justify-between">
              <div>
                <h1 className="text-3xl font-bold text-white mb-2">Trading Dashboard</h1>
                <p className="text-gray-400">Real-time store synchronization with WebGPU acceleration</p>
              </div>
              
              {/* Debug Controls */}
              <div className="flex items-center gap-4">
                <label className="flex items-center gap-2 text-sm text-gray-300">
                  <input
                    type="checkbox"
                    checked={showDebugMode}
                    onChange={(e) => setShowDebugMode(e.target.checked)}
                    className="rounded"
                  />
                  Debug Mode
                </label>
                
                <label className="flex items-center gap-2 text-sm text-gray-300">
                  <input
                    type="checkbox"
                    checked={showSubscriptionInfo}
                    onChange={(e) => setShowSubscriptionInfo(e.target.checked)}
                    className="rounded"
                  />
                  Subscription Info
                </label>
                
                <label className="flex items-center gap-2 text-sm text-gray-300">
                  <input
                    type="checkbox"
                    checked={enableChangeTracking}
                    onChange={(e) => setEnableChangeTracking(e.target.checked)}
                    className="rounded"
                  />
                  Change Tracking
                </label>
              </div>
            </div>
            
            <div className="flex-1 flex gap-6">
              {/* Chart Controls Panel */}
              <div className="w-80 flex-shrink-0 space-y-4">
                <ChartControls 
                  showSubscriptionInfo={showSubscriptionInfo}
                  enableChangeTracking={enableChangeTracking}
                />
                
                {/* Data Fetching Monitor */}
                <DataFetchingMonitor 
                  showDetailedInfo={debugMode}
                  enableManualControls={true}
                  showActivity={debugMode}
                  compactMode={!debugMode}
                />
              </div>
              
              {/* Main Chart Area */}
              <div className="flex-1 flex flex-col">
                <WasmCanvas 
                  enableAutoSync={true}
                  debounceMs={100}
                  showPerformanceOverlay={true}
                  debugMode={debugMode}
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