import { useEffect } from 'react';
import WasmCanvas from '../components/chart/WasmCanvas';

export default function TestNewArchitecture() {
  useEffect(() => {
    console.log('[TestNewArchitecture] Page loaded');
    console.log('[TestNewArchitecture] Using new architecture:', process.env.REACT_APP_USE_NEW_ARCHITECTURE);
  }, []);

  return (
    <div className="h-screen bg-gray-900 flex flex-col">
      <div className="bg-gray-800 text-white p-4 border-b border-gray-700">
        <h1 className="text-2xl font-bold">New Architecture Test</h1>
        <p className="text-gray-400">Testing gpu-charts-wasm (data-manager + renderer + wasm-bridge)</p>
      </div>
      
      <div className="flex-1 p-4">
        <div className="h-full bg-gray-800 rounded-lg overflow-hidden">
          <WasmCanvas
            enableAutoSync={false}
            showPerformanceOverlay={true}
            debugMode={true}
          />
        </div>
      </div>
      
      <div className="bg-gray-800 text-white p-4 border-t border-gray-700">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-gray-400">Status: Testing new architecture</p>
          </div>
          <div className="flex gap-4 text-sm">
            <button
              onClick={() => window.location.reload()}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded transition-colors"
            >
              Reload
            </button>
            <button
              onClick={() => {
                const chartAPI = (window as any).__CHART_INSTANCE__?.chartAPI;
                if (chartAPI) {
                  chartAPI.updateChart('line', 'BTC-USD', Date.now() - 3600000, Date.now());
                }
              }}
              className="px-4 py-2 bg-green-600 hover:bg-green-700 rounded transition-colors"
            >
              Load Test Data
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}