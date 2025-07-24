import React from 'react';
import { useAppStore } from '../../store/useAppStore';

export const SimpleChartControls: React.FC = () => {
  const chartType = useAppStore((state) => state.chartConfig.chartType);
  const setChartType = useAppStore((state) => state.setChartType);
  const candleTimeframe = useAppStore((state) => state.chartConfig.candleTimeframe);
  const setCandleTimeframe = useAppStore((state) => state.setCandleTimeframe);

  return (
    <div className="flex items-center space-x-4" data-testid="chart-controls">
      <div className="flex items-center space-x-2">
        <span className="text-sm text-gray-400">Chart:</span>
        <div className="flex space-x-1">
          <button
            onClick={() => setChartType('line')}
            className={`px-3 py-1 text-xs font-medium rounded transition-colors ${
              chartType === 'line'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-600 text-gray-300 hover:bg-gray-500'
            }`}
          >
            Line
          </button>
          <button
            onClick={() => setChartType('candlestick')}
            className={`px-3 py-1 text-xs font-medium rounded transition-colors ${
              chartType === 'candlestick'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-600 text-gray-300 hover:bg-gray-500'
            }`}
          >
            Candlestick
          </button>
        </div>
      </div>

      {chartType === 'candlestick' && (
        <div className="flex items-center space-x-2">
          <span className="text-sm text-gray-400">Timeframe:</span>
          <select
            value={candleTimeframe}
            onChange={(e) => setCandleTimeframe(Number(e.target.value))}
            className="bg-gray-700 text-white text-sm rounded px-2 py-1 border border-gray-600 focus:border-blue-500 focus:outline-none"
            data-testid="timeframe-select"
          >
            <option value={60}>1 minute</option>
            <option value={300}>5 minutes</option>
            <option value={900}>15 minutes</option>
            <option value={3600}>1 hour</option>
          </select>
        </div>
      )}
      
    </div>
  );
};