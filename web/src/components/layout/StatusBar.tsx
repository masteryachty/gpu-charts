import { useEffect, useState } from 'react';
import { useAppStore } from '../../store/useAppStore';

export default function StatusBar() {
  const { currentSymbol, marketData, isConnected } = useAppStore();
  const [currentTime, setCurrentTime] = useState(new Date());
  const [performanceMetrics, setPerformanceMetrics] = useState({ fps: 0, renderLatency: 0 });

  const symbolData = marketData[currentSymbol];

  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);

    return () => clearInterval(timer);
  }, []);

  // Update performance metrics from global state
  useEffect(() => {
    const updatePerformanceMetrics = () => {
      const globalMetrics = (window as any).__PERFORMANCE_METRICS__;
      const wasmChart = (window as any).__wasmChart;
      const perfMonitor = (window as any).__PERFORMANCE_MONITOR_STATE__;
      
      // Priority: wasmChart > globalMetrics > perfMonitor > defaults
      const fps = wasmChart?.fps || 
                  globalMetrics?.fps || 
                  perfMonitor?.metrics?.fps || 
                  60;
      
      const renderLatency = wasmChart?.renderLatency || 
                           globalMetrics?.renderLatency || 
                           perfMonitor?.metrics?.renderLatency || 
                           16.67;
      
      setPerformanceMetrics({ fps, renderLatency });
    };

    // Update immediately and then every second
    updatePerformanceMetrics();
    const perfTimer = setInterval(updatePerformanceMetrics, 1000);

    return () => clearInterval(perfTimer);
  }, []);

  const formatPrice = (price: number) => {
    return price.toFixed(2);
  };

  const formatChange = (change: number, changePercent: number) => {
    const sign = change >= 0 ? '+' : '';
    const color = change >= 0 ? 'text-accent-green' : 'text-accent-red';
    return (
      <span className={color}>
        {sign}{formatPrice(change)} ({sign}{changePercent.toFixed(2)}%)
      </span>
    );
  };

  const formatVolume = (volume: number) => {
    if (volume >= 1000000) {
      return `${(volume / 1000000).toFixed(1)}M`;
    }
    if (volume >= 1000) {
      return `${(volume / 1000).toFixed(1)}K`;
    }
    return volume.toString();
  };

  return (
    <div className="h-10 bg-bg-secondary border-t border-border flex items-center justify-between px-6 text-sm">
      {/* Market Data */}
      <div className="flex items-center gap-8">
        {symbolData ? (
          <>
            <div className="flex items-center gap-2">
              <span className="text-text-secondary">{currentSymbol}</span>
              <span className="text-text-primary font-mono">
                ${formatPrice(symbolData.price)}
              </span>
              {formatChange(symbolData.change, symbolData.changePercent)}
            </div>
            
            <div className="flex items-center gap-2">
              <span className="text-text-secondary">Vol:</span>
              <span className="text-text-primary font-mono">
                {formatVolume(symbolData.volume)}
              </span>
            </div>
          </>
        ) : (
          <div className="text-text-tertiary">
            No data for {currentSymbol}
          </div>
        )}
      </div>

      {/* Status and Time */}
      <div className="flex items-center gap-8">
        {/* Connection Status */}
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${
            isConnected ? 'bg-accent-green' : 'bg-accent-red'
          }`} />
          <span className="text-text-tertiary">
            {isConnected ? 'Live' : 'Disconnected'}
          </span>
        </div>

        {/* Performance Metrics */}
        <div className="text-text-tertiary font-mono">
          <span className={performanceMetrics.fps < 30 ? 'text-accent-red' : performanceMetrics.fps < 45 ? 'text-yellow-400' : 'text-accent-green'}>
            {Math.round(performanceMetrics.fps)} FPS
          </span>
          {' | '}
          <span className={performanceMetrics.renderLatency > 50 ? 'text-accent-red' : performanceMetrics.renderLatency > 25 ? 'text-yellow-400' : 'text-accent-green'}>
            {performanceMetrics.renderLatency.toFixed(1)}ms
          </span>
        </div>

        {/* Current Time */}
        <div className="text-text-tertiary font-mono">
          {currentTime.toLocaleTimeString()}
        </div>
      </div>
    </div>
  );
}