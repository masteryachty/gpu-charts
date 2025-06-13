import { useCallback, useEffect, useState } from 'react';
// import { useAutonomousDataFetching } from '../../hooks/useAutonomousDataFetching';

/**
 * Data Fetching Monitor Component
 * 
 * Provides real-time monitoring and control of the autonomous data fetching system,
 * including metrics, cache status, and manual controls.
 */

interface DataFetchingMonitorProps {
  /** Show detailed metrics and cache information */
  showDetailedInfo?: boolean;
  
  /** Enable manual fetching controls */
  enableManualControls?: boolean;
  
  /** Show real-time request activity */
  showActivity?: boolean;
  
  /** Compact mode for smaller displays */
  compactMode?: boolean;
}

export default function DataFetchingMonitor({
  showDetailedInfo = true,
  enableManualControls = true,
  showActivity = true,
  compactMode = false
}: DataFetchingMonitorProps) {
  // TODO: Instead of creating its own service, this should get a reference to the 
  // existing service from useWasmChart. For now, disable to prevent conflicts.
  // const [fetchingState, fetchingAPI] = useAutonomousDataFetching({
  //   enableAutoFetch: false, // Monitor only - don't create competing fetching service
  //   enablePrefetch: false,
  //   debounceMs: 300,
  // });
  
  // Temporary mock state to prevent errors
  const fetchingState = {
    service: null,
    isLoading: false,
    isFetching: false,
    isBackgroundFetching: false,
    lastFetch: null,
    error: null,
    lastError: null,
    metrics: {
      totalRequests: 0,
      successfulRequests: 0,
      failedRequests: 0,
      cacheHits: 0,
      cacheMisses: 0,
      averageLatency: 0,
      backgroundFetches: 0,
      prefetches: 0,
    },
    pendingRequests: 0,
    cacheStats: { size: 0, hitRate: 0, entries: [] }
  };
  
  const fetchingAPI = {
    fetchData: async () => ({ success: false, data: null, metadata: null, error: 'Monitor disabled' }),
    refreshData: async () => ({ success: false, data: null, metadata: null, error: 'Monitor disabled' }),
    prefetchData: async () => {},
    clearCache: () => {},
    getDetailedMetrics: () => fetchingState.metrics,
    setAutoFetch: () => {},
    configureService: () => {}
  };

  const [activityLog, setActivityLog] = useState<Array<{
    id: string;
    type: 'fetch' | 'cache_hit' | 'error' | 'prefetch';
    symbol: string;
    timestamp: number;
    details: string;
  }>>([]);

  const [autoFetchEnabled, setAutoFetchEnabled] = useState(true);

  // Track activity for the activity log
  useEffect(() => {
    if (!showActivity) return;

    if (fetchingState.lastFetch) {
      const newActivity = {
        id: Math.random().toString(36),
        type: fetchingState.lastFetch.fromCache ? 'cache_hit' as const : 'fetch' as const,
        symbol: fetchingState.lastFetch.symbol,
        timestamp: fetchingState.lastFetch.timestamp,
        details: `${fetchingState.lastFetch.recordCount} records ${fetchingState.lastFetch.fromCache ? '(cached)' : '(fetched)'}`
      };

      setActivityLog(prev => [newActivity, ...prev.slice(0, 19)]); // Keep last 20 activities
    }

    if (fetchingState.error && fetchingState.lastError) {
      const errorActivity = {
        id: Math.random().toString(36),
        type: 'error' as const,
        symbol: 'unknown',
        timestamp: Date.now(),
        details: fetchingState.error
      };

      setActivityLog(prev => [errorActivity, ...prev.slice(0, 19)]);
    }
  }, [fetchingState.lastFetch, fetchingState.error, fetchingState.lastError, showActivity]);

  // Handle auto-fetch toggle
  const handleAutoFetchToggle = useCallback(() => {
    const newState = !autoFetchEnabled;
    setAutoFetchEnabled(newState);
    fetchingAPI.setAutoFetch(newState);
  }, [autoFetchEnabled, fetchingAPI]);

  // Handle manual refresh
  const handleManualRefresh = useCallback(async () => {
    console.log('[DataFetchingMonitor] Manual refresh triggered');
    await fetchingAPI.refreshData();
  }, [fetchingAPI]);

  // Handle cache clear
  const handleClearCache = useCallback(() => {
    console.log('[DataFetchingMonitor] Cache cleared by user');
    fetchingAPI.clearCache();
  }, [fetchingAPI]);

  // Handle prefetch popular symbols
  const handlePrefetchPopular = useCallback(async () => {
    console.log('[DataFetchingMonitor] Prefetching popular symbols');
    const popularSymbols = ['BTC-USD', 'ETH-USD', 'ADA-USD'];
    
    for (const symbol of popularSymbols) {
      await fetchingAPI.prefetchData(symbol, '1h');
    }
  }, [fetchingAPI]);

  // Calculate cache hit rate
  const cacheHitRate = fetchingState.metrics.cacheHits / 
    (fetchingState.metrics.cacheHits + fetchingState.metrics.cacheMisses) * 100 || 0;

  // Calculate success rate
  const successRate = fetchingState.metrics.successfulRequests / 
    fetchingState.metrics.totalRequests * 100 || 0;

  // Format file size
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  };

  // Format timestamp
  const formatTime = (timestamp: number): string => {
    return new Date(timestamp).toLocaleTimeString();
  };

  if (compactMode) {
    return (
      <div className="bg-gray-800 border border-gray-600 rounded p-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="flex items-center gap-2">
              <div className={`w-2 h-2 rounded-full ${
                fetchingState.isFetching ? 'bg-blue-500 animate-pulse' : 'bg-green-500'
              }`} />
              <span className="text-white text-sm">
                {fetchingState.isFetching ? 'Fetching...' : 'Ready'}
              </span>
            </div>
            
            <div className="text-gray-400 text-xs">
              {fetchingState.metrics.totalRequests} requests | {cacheHitRate.toFixed(1)}% cache hit
            </div>
          </div>
          
          {enableManualControls && (
            <button
              onClick={handleManualRefresh}
              disabled={fetchingState.isFetching}
              className="px-2 py-1 bg-blue-600 text-white text-xs rounded hover:bg-blue-700 disabled:opacity-50"
            >
              Refresh
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="bg-gray-800 border border-gray-600 rounded-lg p-4 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-white font-semibold">Data Fetching Monitor</h3>
        <div className="flex items-center gap-2">
          <div className={`w-3 h-3 rounded-full ${
            fetchingState.isFetching ? 'bg-blue-500 animate-pulse' : 
            fetchingState.error ? 'bg-red-500' : 'bg-green-500'
          }`} />
          <span className="text-gray-400 text-sm">
            {fetchingState.isFetching ? 'Active' : 
             fetchingState.error ? 'Error' : 'Ready'}
          </span>
        </div>
      </div>

      {/* Current Status */}
      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-2">
          <div className="text-gray-300 text-sm">
            <span className="text-gray-500">Pending:</span> {fetchingState.pendingRequests}
          </div>
          <div className="text-gray-300 text-sm">
            <span className="text-gray-500">Auto-fetch:</span> 
            <span className={autoFetchEnabled ? 'text-green-500' : 'text-red-500'}>
              {autoFetchEnabled ? ' Enabled' : ' Disabled'}
            </span>
          </div>
          {fetchingState.isBackgroundFetching && (
            <div className="text-yellow-500 text-sm">Background fetching...</div>
          )}
        </div>
        
        <div className="space-y-2">
          {fetchingState.lastFetch && (
            <>
              <div className="text-gray-300 text-sm">
                <span className="text-gray-500">Last:</span> {fetchingState.lastFetch.symbol}
              </div>
              <div className="text-gray-300 text-sm">
                <span className="text-gray-500">Records:</span> {fetchingState.lastFetch.recordCount.toLocaleString()}
              </div>
            </>
          )}
        </div>
      </div>

      {/* Error Display */}
      {fetchingState.error && (
        <div className="bg-red-900/30 border border-red-600 rounded p-3">
          <div className="text-red-400 text-sm font-medium mb-1">Error</div>
          <div className="text-red-300 text-xs">{fetchingState.error}</div>
        </div>
      )}

      {/* Metrics */}
      {showDetailedInfo && (
        <div className="border-t border-gray-600 pt-4">
          <h4 className="text-gray-300 text-sm font-medium mb-3">Performance Metrics</h4>
          
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Total Requests:</span>
                <span className="text-white">{fetchingState.metrics.totalRequests}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Success Rate:</span>
                <span className={successRate >= 95 ? 'text-green-500' : successRate >= 80 ? 'text-yellow-500' : 'text-red-500'}>
                  {successRate.toFixed(1)}%
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Avg Latency:</span>
                <span className="text-white">{fetchingState.metrics.averageLatency.toFixed(0)}ms</span>
              </div>
            </div>
            
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Cache Hit Rate:</span>
                <span className={cacheHitRate >= 70 ? 'text-green-500' : cacheHitRate >= 40 ? 'text-yellow-500' : 'text-red-500'}>
                  {cacheHitRate.toFixed(1)}%
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Background Fetches:</span>
                <span className="text-blue-400">{fetchingState.metrics.backgroundFetches}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Prefetches:</span>
                <span className="text-purple-400">{fetchingState.metrics.prefetches}</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Cache Information */}
      {showDetailedInfo && (
        <div className="border-t border-gray-600 pt-4">
          <h4 className="text-gray-300 text-sm font-medium mb-3">Cache Status</h4>
          
          <div className="space-y-2">
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Cache Size:</span>
              <span className="text-white">
                {fetchingState.cacheStats.size} / {fetchingState.cacheStats.maxSize}
              </span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-400">Total Size:</span>
              <span className="text-white">{formatBytes(fetchingState.cacheStats.totalSize)}</span>
            </div>
            
            {/* Cache usage bar */}
            <div className="mt-2">
              <div className="w-full bg-gray-700 rounded-full h-2">
                <div 
                  className="bg-blue-600 h-2 rounded-full transition-all duration-300"
                  style={{ 
                    width: `${(fetchingState.cacheStats.size / fetchingState.cacheStats.maxSize) * 100}%` 
                  }}
                />
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Manual Controls */}
      {enableManualControls && (
        <div className="border-t border-gray-600 pt-4">
          <h4 className="text-gray-300 text-sm font-medium mb-3">Controls</h4>
          
          <div className="grid grid-cols-2 gap-2">
            <button
              onClick={handleAutoFetchToggle}
              className={`px-3 py-2 text-sm rounded transition-colors ${
                autoFetchEnabled 
                  ? 'bg-green-600 text-white hover:bg-green-700'
                  : 'bg-gray-600 text-white hover:bg-gray-700'
              }`}
            >
              {autoFetchEnabled ? 'Disable Auto' : 'Enable Auto'}
            </button>
            
            <button
              onClick={handleManualRefresh}
              disabled={fetchingState.isFetching}
              className="px-3 py-2 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 disabled:opacity-50 transition-colors"
            >
              Manual Refresh
            </button>
            
            <button
              onClick={handleClearCache}
              className="px-3 py-2 bg-red-600 text-white text-sm rounded hover:bg-red-700 transition-colors"
            >
              Clear Cache
            </button>
            
            <button
              onClick={handlePrefetchPopular}
              disabled={fetchingState.isFetching}
              className="px-3 py-2 bg-purple-600 text-white text-sm rounded hover:bg-purple-700 disabled:opacity-50 transition-colors"
            >
              Prefetch Popular
            </button>
          </div>
        </div>
      )}

      {/* Activity Log */}
      {showActivity && activityLog.length > 0 && (
        <div className="border-t border-gray-600 pt-4">
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-gray-300 text-sm font-medium">Recent Activity</h4>
            <button
              onClick={() => setActivityLog([])}
              className="text-gray-400 hover:text-white text-xs"
            >
              Clear
            </button>
          </div>
          
          <div className="max-h-32 overflow-y-auto space-y-1">
            {activityLog.slice(0, 8).map((activity) => (
              <div key={activity.id} className="flex items-center justify-between text-xs">
                <div className="flex items-center gap-2">
                  <div className={`w-1.5 h-1.5 rounded-full ${
                    activity.type === 'fetch' ? 'bg-blue-500' :
                    activity.type === 'cache_hit' ? 'bg-green-500' :
                    activity.type === 'prefetch' ? 'bg-purple-500' :
                    'bg-red-500'
                  }`} />
                  <span className="text-gray-300">{activity.symbol}</span>
                  <span className="text-gray-500">{activity.details}</span>
                </div>
                <span className="text-gray-500">{formatTime(activity.timestamp)}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}