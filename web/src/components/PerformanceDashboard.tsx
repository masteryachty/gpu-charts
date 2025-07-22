import React, { useState, useEffect, useRef } from 'react';
import { Chart } from '@pkg/GPU_charting';

interface PerformanceDashboardProps {
  chart: Chart | null;
  compact?: boolean;
}

interface PerformanceMetrics {
  fps: number;
  frameTime: number;
  memoryUsage: number;
  gpuTime: number;
  cpuTime: number;
  drawCalls: number;
  vertices: number;
  triangles: number;
}

interface HistoricalData {
  timestamp: number;
  fps: number;
  frameTime: number;
  memoryUsage: number;
}

const PerformanceDashboard: React.FC<PerformanceDashboardProps> = ({ chart, compact = false }) => {
  const [metrics, setMetrics] = useState<PerformanceMetrics>({
    fps: 0,
    frameTime: 0,
    memoryUsage: 0,
    gpuTime: 0,
    cpuTime: 0,
    drawCalls: 0,
    vertices: 0,
    triangles: 0
  });

  const [history, setHistory] = useState<HistoricalData[]>([]);
  const [isMonitoring, setIsMonitoring] = useState(true);
  const intervalRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    if (!chart || !isMonitoring) {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      return;
    }

    // Update metrics every 100ms
    intervalRef.current = setInterval(() => {
      updateMetrics();
    }, 100);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [chart, isMonitoring]);

  const updateMetrics = () => {
    if (!chart) return;

    // Get browser performance metrics
    const performance = window.performance as any;
    const memory = performance.memory;
    
    // Calculate FPS from frame timing
    const entries = performance.getEntriesByType('measure');
    const frameEntries = entries.filter((e: any) => e.name.includes('frame'));
    const avgFrameTime = frameEntries.length > 0 
      ? frameEntries.reduce((sum: number, e: any) => sum + e.duration, 0) / frameEntries.length
      : 16.67; // Default to 60 FPS
    
    const fps = Math.round(1000 / avgFrameTime);
    const memoryUsage = memory ? memory.usedJSHeapSize / (1024 * 1024) : 0;

    const newMetrics: PerformanceMetrics = {
      fps,
      frameTime: avgFrameTime,
      memoryUsage,
      gpuTime: 0, // TODO: Get from WebGPU timing queries
      cpuTime: 0, // TODO: Calculate from performance API
      drawCalls: 0, // TODO: Get from chart render stats
      vertices: 0, // TODO: Get from chart render stats
      triangles: 0 // TODO: Get from chart render stats
    };

    setMetrics(newMetrics);

    // Update history (keep last 60 seconds)
    setHistory(prev => {
      const now = Date.now();
      const newHistory = [...prev, {
        timestamp: now,
        fps,
        frameTime: avgFrameTime,
        memoryUsage
      }];
      
      // Keep only last 60 seconds
      return newHistory.filter(h => h.timestamp > now - 60000);
    });
  };

  const getMetricColor = (value: number, type: 'fps' | 'frameTime' | 'memory') => {
    switch (type) {
      case 'fps':
        if (value >= 55) return 'text-green-500';
        if (value >= 30) return 'text-yellow-500';
        return 'text-red-500';
      case 'frameTime':
        if (value <= 16.67) return 'text-green-500';
        if (value <= 33.33) return 'text-yellow-500';
        return 'text-red-500';
      case 'memory':
        if (value <= 100) return 'text-green-500';
        if (value <= 200) return 'text-yellow-500';
        return 'text-red-500';
      default:
        return 'text-gray-400';
    }
  };

  const renderSparkline = (data: number[], max: number, color: string) => {
    if (data.length < 2) return null;

    const width = 100;
    const height = 30;
    const points = data.map((value, index) => {
      const x = (index / (data.length - 1)) * width;
      const y = height - (value / max) * height;
      return `${x},${y}`;
    }).join(' ');

    return (
      <svg width={width} height={height} className="inline-block ml-2">
        <polyline
          points={points}
          fill="none"
          stroke={color}
          strokeWidth="2"
        />
      </svg>
    );
  };

  if (!chart) {
    return (
      <div className="p-4 border rounded-lg bg-gray-50">
        <p className="text-gray-500">Performance monitoring inactive</p>
      </div>
    );
  }

  if (compact) {
    return (
      <div className="bg-gray-800 text-white p-2 rounded-lg text-xs">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-4">
            <span className={getMetricColor(metrics.fps, 'fps')}>
              {metrics.fps} FPS
            </span>
            <span className={getMetricColor(metrics.frameTime, 'frameTime')}>
              {metrics.frameTime.toFixed(1)}ms
            </span>
            <span className={getMetricColor(metrics.memoryUsage, 'memory')}>
              {metrics.memoryUsage.toFixed(0)}MB
            </span>
          </div>
          <button
            onClick={() => setIsMonitoring(!isMonitoring)}
            className="text-gray-400 hover:text-white"
          >
            {isMonitoring ? '⏸' : '▶'}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="p-4 border rounded-lg bg-white shadow-sm space-y-4">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold">Performance Metrics</h2>
        <div className="flex items-center space-x-2">
          <button
            onClick={() => setIsMonitoring(!isMonitoring)}
            className={`px-3 py-1 rounded text-sm ${
              isMonitoring 
                ? 'bg-red-500 text-white hover:bg-red-600' 
                : 'bg-green-500 text-white hover:bg-green-600'
            }`}
          >
            {isMonitoring ? 'Stop' : 'Start'} Monitoring
          </button>
        </div>
      </div>

      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        {/* FPS */}
        <div className="bg-gray-50 p-3 rounded">
          <div className="text-sm text-gray-600 mb-1">Frame Rate</div>
          <div className={`text-2xl font-bold ${getMetricColor(metrics.fps, 'fps')}`}>
            {metrics.fps}
            <span className="text-sm font-normal ml-1">FPS</span>
          </div>
          {history.length > 10 && renderSparkline(
            history.slice(-30).map(h => h.fps),
            120,
            '#10b981'
          )}
        </div>

        {/* Frame Time */}
        <div className="bg-gray-50 p-3 rounded">
          <div className="text-sm text-gray-600 mb-1">Frame Time</div>
          <div className={`text-2xl font-bold ${getMetricColor(metrics.frameTime, 'frameTime')}`}>
            {metrics.frameTime.toFixed(1)}
            <span className="text-sm font-normal ml-1">ms</span>
          </div>
          {history.length > 10 && renderSparkline(
            history.slice(-30).map(h => h.frameTime),
            50,
            '#f59e0b'
          )}
        </div>

        {/* Memory Usage */}
        <div className="bg-gray-50 p-3 rounded">
          <div className="text-sm text-gray-600 mb-1">Memory Usage</div>
          <div className={`text-2xl font-bold ${getMetricColor(metrics.memoryUsage, 'memory')}`}>
            {metrics.memoryUsage.toFixed(0)}
            <span className="text-sm font-normal ml-1">MB</span>
          </div>
          {history.length > 10 && renderSparkline(
            history.slice(-30).map(h => h.memoryUsage),
            300,
            '#3b82f6'
          )}
        </div>

        {/* GPU Time */}
        <div className="bg-gray-50 p-3 rounded">
          <div className="text-sm text-gray-600 mb-1">GPU Time</div>
          <div className="text-2xl font-bold text-gray-400">
            {metrics.gpuTime.toFixed(1)}
            <span className="text-sm font-normal ml-1">ms</span>
          </div>
        </div>
      </div>

      {/* Additional Metrics */}
      <div className="border-t pt-4">
        <h3 className="text-sm font-medium text-gray-700 mb-2">Render Statistics</h3>
        <div className="grid grid-cols-3 gap-4 text-sm">
          <div>
            <span className="text-gray-600">Draw Calls:</span>
            <span className="ml-2 font-medium">{metrics.drawCalls}</span>
          </div>
          <div>
            <span className="text-gray-600">Vertices:</span>
            <span className="ml-2 font-medium">{metrics.vertices.toLocaleString()}</span>
          </div>
          <div>
            <span className="text-gray-600">Triangles:</span>
            <span className="ml-2 font-medium">{metrics.triangles.toLocaleString()}</span>
          </div>
        </div>
      </div>

      {/* Performance Tips */}
      <div className="border-t pt-4">
        <h3 className="text-sm font-medium text-gray-700 mb-2">Performance Tips</h3>
        <ul className="text-xs text-gray-600 space-y-1">
          {metrics.fps < 30 && (
            <li className="text-red-600">• Low FPS detected. Consider reducing data points or disabling features.</li>
          )}
          {metrics.memoryUsage > 200 && (
            <li className="text-yellow-600">• High memory usage. Consider clearing old data or reducing buffer sizes.</li>
          )}
          {metrics.frameTime > 33.33 && (
            <li className="text-yellow-600">• High frame time. Consider enabling performance optimizations.</li>
          )}
          {metrics.fps >= 55 && metrics.memoryUsage <= 100 && (
            <li className="text-green-600">• Performance is optimal!</li>
          )}
        </ul>
      </div>
    </div>
  );
};

export default PerformanceDashboard;