import type { ChartConfig, MarketData } from '../types';
import { handleDataError, handleNetworkError, ERROR_CODES } from '../errors';

/**
 * Autonomous Data Fetching Service
 * 
 * Intelligent data fetching system that automatically responds to store state changes,
 * implements background loading, caching, and real-time data streams with optimization.
 */

export interface DataFetchRequest {
  symbol: string;
  startTime: number;
  endTime: number;
  timeframe: string;
  columns: string[];
  priority: 'low' | 'normal' | 'high' | 'critical';
  reason: 'user_action' | 'auto_sync' | 'prefetch' | 'real_time';
}

export interface DataFetchResponse {
  success: boolean;
  data?: ArrayBuffer;
  metadata?: {
    symbol: string;
    timeRange: [number, number];
    recordCount: number;
    fetchTime: number;
    cacheHit: boolean;
  };
  error?: string;
  retryAfter?: number;
}

export interface CacheEntry {
  data: ArrayBuffer;
  metadata: {
    symbol: string;
    timeRange: [number, number];
    fetchTime: number;
    lastAccess: number;
    hitCount: number;
  };
  expiresAt: number;
}

export interface FetchingMetrics {
  totalRequests: number;
  successfulRequests: number;
  failedRequests: number;
  cacheHits: number;
  cacheMisses: number;
  averageLatency: number;
  backgroundFetches: number;
  prefetches: number;
}

export interface DataFetchingConfig {
  // Cache configuration
  maxCacheSize: number;          // Maximum cache entries
  cacheExpiryMs: number;         // Cache expiry time
  prefetchEnabled: boolean;      // Enable predictive prefetching
  
  // Request optimization
  maxConcurrentRequests: number; // Max parallel requests
  requestTimeoutMs: number;      // Request timeout
  retryAttempts: number;         // Max retry attempts
  retryDelayMs: number;          // Base retry delay
  
  // Background fetching
  backgroundFetchEnabled: boolean; // Enable background updates
  backgroundUpdateIntervalMs: number; // Background update frequency
  
  // Real-time streaming
  streamingEnabled: boolean;     // Enable WebSocket streaming
  streamBufferSize: number;      // Stream buffer size
  
  // Predictive fetching
  prefetchTimeMultiplier: number; // How much extra time to fetch
  prefetchPopularSymbols: string[]; // Symbols to prefetch
}

/**
 * Autonomous Data Fetching Service
 * 
 * Features:
 * - Intelligent cache management with LRU eviction
 * - Background data fetching and updates
 * - Predictive prefetching based on user patterns
 * - Real-time data streaming integration
 * - Request deduplication and batching
 * - Performance metrics and monitoring
 */
export class DataFetchingService {
  private config: DataFetchingConfig;
  private cache = new Map<string, CacheEntry>();
  private activeRequests = new Map<string, Promise<DataFetchResponse>>();
  private requestQueue: DataFetchRequest[] = [];
  private metrics: FetchingMetrics;
  private subscribers = new Set<(response: DataFetchResponse) => void>();
  private backgroundIntervalId?: NodeJS.Timeout;
  private websocket?: WebSocket;
  
  // User behavior tracking for predictive fetching
  private accessPatterns = new Map<string, { symbol: string; timeframe: string; count: number; lastAccess: number }>();
  private prefetchCandidates = new Set<string>();

  constructor(config: Partial<DataFetchingConfig> = {}) {
    this.config = {
      maxCacheSize: 100,
      cacheExpiryMs: 5 * 60 * 1000, // 5 minutes
      prefetchEnabled: true,
      maxConcurrentRequests: 6,
      requestTimeoutMs: 10000,
      retryAttempts: 3,
      retryDelayMs: 1000,
      backgroundFetchEnabled: true,
      backgroundUpdateIntervalMs: 30000, // 30 seconds
      streamingEnabled: false,
      streamBufferSize: 1000,
      prefetchTimeMultiplier: 1.5,
      prefetchPopularSymbols: ['BTC-USD', 'ETH-USD', 'ADA-USD'],
      ...config,
    };

    this.metrics = {
      totalRequests: 0,
      successfulRequests: 0,
      failedRequests: 0,
      cacheHits: 0,
      cacheMisses: 0,
      averageLatency: 0,
      backgroundFetches: 0,
      prefetches: 0,
    };

    this.startBackgroundServices();
  }

  /**
   * Primary data fetching method - intelligently handles requests
   */
  async fetchData(request: DataFetchRequest): Promise<DataFetchResponse> {
    console.log('[DataFetchingService] Fetch request:', {
      symbol: request.symbol,
      timeRange: [request.startTime, request.endTime],
      reason: request.reason,
      priority: request.priority
    });

    // Track user access patterns for predictive fetching
    this.trackAccessPattern(request);

    // Generate cache key
    const cacheKey = this.generateCacheKey(request);

    // Check cache first
    const cachedData = this.getCachedData(cacheKey);
    if (cachedData) {
      console.log('[DataFetchingService] Cache hit for:', cacheKey);
      this.metrics.cacheHits++;
      return {
        success: true,
        data: cachedData.data,
        metadata: {
          ...cachedData.metadata,
          cacheHit: true,
        }
      };
    }

    this.metrics.cacheMisses++;

    // Check for active request (deduplication)
    const activeRequest = this.activeRequests.get(cacheKey);
    if (activeRequest) {
      console.log('[DataFetchingService] Deduplicating request for:', cacheKey);
      return activeRequest;
    }

    // Create new request
    const fetchPromise = this.executeDataFetch(request, cacheKey);
    this.activeRequests.set(cacheKey, fetchPromise);

    try {
      const response = await fetchPromise;
      
      // Cache successful responses
      if (response.success && response.data) {
        this.cacheData(cacheKey, response);
      }

      // Trigger predictive prefetching
      if (response.success && this.config.prefetchEnabled) {
        this.triggerPredictivePrefetch(request);
      }

      return response;
    } finally {
      this.activeRequests.delete(cacheKey);
    }
  }

  /**
   * Execute the actual HTTP data fetch with retry logic
   */
  private async executeDataFetch(request: DataFetchRequest, cacheKey: string): Promise<DataFetchResponse> {
    const startTime = performance.now();
    this.metrics.totalRequests++;

    for (let attempt = 1; attempt <= this.config.retryAttempts; attempt++) {
      try {
        console.log(`[DataFetchingService] Fetching data (attempt ${attempt}):`, cacheKey);

        // Construct data server URL
        const url = this.buildDataServerUrl(request);
        
        // Execute fetch with timeout
        const response = await this.fetchWithTimeout(url, this.config.requestTimeoutMs);
        
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }

        // Get response data as ArrayBuffer
        const data = await response.arrayBuffer();
        const endTime = performance.now();
        const fetchTime = endTime - startTime;

        // Update metrics
        this.metrics.successfulRequests++;
        this.updateLatencyMetric(fetchTime);

        // Track background vs user-initiated fetches
        if (request.reason === 'prefetch' || request.reason === 'auto_sync') {
          this.metrics.backgroundFetches++;
        }
        if (request.reason === 'prefetch') {
          this.metrics.prefetches++;
        }

        console.log(`[DataFetchingService] Successfully fetched ${data.byteLength} bytes in ${fetchTime.toFixed(1)}ms`);

        return {
          success: true,
          data,
          metadata: {
            symbol: request.symbol,
            timeRange: [request.startTime, request.endTime],
            recordCount: Math.floor(data.byteLength / 4), // Assuming 4-byte records
            fetchTime,
            cacheHit: false,
          }
        };

      } catch (error) {
        console.warn(`[DataFetchingService] Fetch attempt ${attempt} failed:`, error);
        
        if (attempt === this.config.retryAttempts) {
          this.metrics.failedRequests++;
          
          // Report to error handling system on final failure
          await handleDataError(
            ERROR_CODES.DATA_FETCH_FAILED,
            `Failed to fetch data for ${request.symbol} after ${attempt} attempts: ${error}`,
            {
              endpoint: this.buildDataServerUrl(request),
              requestId: cacheKey,
              retryable: false,
              retryAfter: this.config.retryDelayMs * Math.pow(2, attempt),
              symbol: request.symbol,
              timeframe: request.timeframe,
              attempts: attempt
            }
          );
          
          return {
            success: false,
            error: `Failed after ${attempt} attempts: ${error}`,
            retryAfter: this.config.retryDelayMs * Math.pow(2, attempt)
          };
        }

        // Exponential backoff
        await this.delay(this.config.retryDelayMs * Math.pow(2, attempt - 1));
      }
    }

    return {
      success: false,
      error: 'Unexpected error in fetch execution'
    };
  }

  /**
   * Build data server URL from request parameters
   */
  private buildDataServerUrl(request: DataFetchRequest): string {
    const baseUrl = 'https://localhost:8443/api/data';
    const params = new URLSearchParams({
      symbol: request.symbol,
      type: 'MD',
      start: request.startTime.toString(),
      end: request.endTime.toString(),
      columns: request.columns.join(',')
    });

    return `${baseUrl}?${params.toString()}`;
  }

  /**
   * Fetch with timeout support
   */
  private async fetchWithTimeout(url: string, timeoutMs: number): Promise<Response> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetch(url, {
        signal: controller.signal,
        method: 'GET',
        headers: {
          'Accept': 'application/octet-stream'
        }
      });
      return response;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Generate cache key from request parameters
   */
  private generateCacheKey(request: DataFetchRequest): string {
    return `${request.symbol}_${request.startTime}_${request.endTime}_${request.timeframe}_${request.columns.join(',')}`;
  }

  /**
   * Get cached data if available and not expired
   */
  private getCachedData(cacheKey: string): CacheEntry | null {
    const entry = this.cache.get(cacheKey);
    if (!entry) return null;

    // Check expiry
    if (Date.now() > entry.expiresAt) {
      this.cache.delete(cacheKey);
      return null;
    }

    // Update access statistics
    entry.metadata.lastAccess = Date.now();
    entry.metadata.hitCount++;

    return entry;
  }

  /**
   * Cache successful data fetch with LRU eviction
   */
  private cacheData(cacheKey: string, response: DataFetchResponse): void {
    if (!response.data || !response.metadata) return;

    // Implement LRU eviction if cache is full
    if (this.cache.size >= this.config.maxCacheSize) {
      this.evictLRUEntries();
    }

    const entry: CacheEntry = {
      data: response.data,
      metadata: {
        symbol: response.metadata.symbol,
        timeRange: response.metadata.timeRange,
        fetchTime: response.metadata.fetchTime,
        lastAccess: Date.now(),
        hitCount: 0,
      },
      expiresAt: Date.now() + this.config.cacheExpiryMs,
    };

    this.cache.set(cacheKey, entry);
    console.log(`[DataFetchingService] Cached data for: ${cacheKey} (${this.cache.size}/${this.config.maxCacheSize})`);
  }

  /**
   * Evict least recently used cache entries
   */
  private evictLRUEntries(): void {
    const entries = Array.from(this.cache.entries());
    entries.sort((a, b) => a[1].metadata.lastAccess - b[1].metadata.lastAccess);

    // Remove oldest 25% of entries
    const toRemove = Math.max(1, Math.floor(this.config.maxCacheSize * 0.25));
    for (let i = 0; i < toRemove; i++) {
      this.cache.delete(entries[i][0]);
    }

    console.log(`[DataFetchingService] Evicted ${toRemove} LRU cache entries`);
  }

  /**
   * Track user access patterns for predictive fetching
   */
  private trackAccessPattern(request: DataFetchRequest): void {
    const key = `${request.symbol}_${request.timeframe}`;
    const existing = this.accessPatterns.get(key);

    if (existing) {
      existing.count++;
      existing.lastAccess = Date.now();
    } else {
      this.accessPatterns.set(key, {
        symbol: request.symbol,
        timeframe: request.timeframe,
        count: 1,
        lastAccess: Date.now()
      });
    }

    // Mark popular patterns as prefetch candidates
    const pattern = this.accessPatterns.get(key);
    if (pattern && pattern.count >= 3) {
      this.prefetchCandidates.add(key);
    }
  }

  /**
   * Trigger predictive prefetching based on current request
   */
  private async triggerPredictivePrefetch(request: DataFetchRequest): Promise<void> {
    if (!this.config.prefetchEnabled) return;

    console.log('[DataFetchingService] Analyzing prefetch opportunities...');

    // Prefetch adjacent time ranges
    const timeRange = request.endTime - request.startTime;
    const extendedRange = Math.floor(timeRange * this.config.prefetchTimeMultiplier);

    // Prefetch future data
    const futureRequest: DataFetchRequest = {
      ...request,
      startTime: request.endTime,
      endTime: request.endTime + extendedRange,
      priority: 'low',
      reason: 'prefetch'
    };

    // Prefetch past data
    const pastRequest: DataFetchRequest = {
      ...request,
      startTime: request.startTime - extendedRange,
      endTime: request.startTime,
      priority: 'low',
      reason: 'prefetch'
    };

    // Queue background prefetch requests
    setTimeout(() => {
      this.queueRequest(futureRequest);
      this.queueRequest(pastRequest);
    }, 1000); // Delay to not interfere with current request
  }

  /**
   * Queue request for background processing
   */
  private queueRequest(request: DataFetchRequest): void {
    this.requestQueue.push(request);
    this.processRequestQueue();
  }

  /**
   * Process background request queue
   */
  private async processRequestQueue(): Promise<void> {
    if (this.activeRequests.size >= this.config.maxConcurrentRequests) return;

    const request = this.requestQueue.shift();
    if (!request) return;

    console.log('[DataFetchingService] Processing background request:', request.reason);

    try {
      await this.fetchData(request);
    } catch (error) {
      console.warn('[DataFetchingService] Background request failed:', error);
    }

    // Continue processing queue
    if (this.requestQueue.length > 0) {
      setTimeout(() => this.processRequestQueue(), 100);
    }
  }

  /**
   * Start background services
   */
  private startBackgroundServices(): void {
    if (this.config.backgroundFetchEnabled) {
      this.backgroundIntervalId = setInterval(() => {
        this.performBackgroundMaintenance();
      }, this.config.backgroundUpdateIntervalMs);
    }
  }

  /**
   * Perform background maintenance tasks
   */
  private performBackgroundMaintenance(): void {
    console.log('[DataFetchingService] Running background maintenance...');
    
    // Clean expired cache entries
    this.cleanExpiredCache();
    
    // Prefetch popular symbols
    this.prefetchPopularData();
    
    // Update access pattern analysis
    this.analyzeAccessPatterns();
  }

  /**
   * Clean expired cache entries
   */
  private cleanExpiredCache(): void {
    const now = Date.now();
    let cleaned = 0;

    for (const [key, entry] of this.cache.entries()) {
      if (now > entry.expiresAt) {
        this.cache.delete(key);
        cleaned++;
      }
    }

    if (cleaned > 0) {
      console.log(`[DataFetchingService] Cleaned ${cleaned} expired cache entries`);
    }
  }

  /**
   * Prefetch popular symbols data
   */
  private prefetchPopularData(): void {
    if (!this.config.prefetchEnabled) return;

    const now = Math.floor(Date.now() / 1000);
    const oneHourAgo = now - 3600;

    for (const symbol of this.config.prefetchPopularSymbols) {
      const request: DataFetchRequest = {
        symbol,
        startTime: oneHourAgo,
        endTime: now,
        timeframe: '1h',
        columns: ['time', 'best_bid', 'best_ask'],
        priority: 'low',
        reason: 'prefetch'
      };

      this.queueRequest(request);
    }
  }

  /**
   * Analyze access patterns for optimization
   */
  private analyzeAccessPatterns(): void {
    const now = Date.now();
    const oneHourAgo = now - 3600000;

    // Remove old access patterns
    for (const [key, pattern] of this.accessPatterns.entries()) {
      if (pattern.lastAccess < oneHourAgo) {
        this.accessPatterns.delete(key);
        this.prefetchCandidates.delete(key);
      }
    }

    console.log(`[DataFetchingService] Active patterns: ${this.accessPatterns.size}, Prefetch candidates: ${this.prefetchCandidates.size}`);
  }

  /**
   * Subscribe to data fetch events
   */
  subscribe(callback: (response: DataFetchResponse) => void): () => void {
    this.subscribers.add(callback);
    return () => this.subscribers.delete(callback);
  }

  /**
   * Get current fetching metrics
   */
  getMetrics(): FetchingMetrics {
    return { ...this.metrics };
  }

  /**
   * Get cache statistics
   */
  getCacheStats() {
    const entries = Array.from(this.cache.values());
    return {
      size: this.cache.size,
      maxSize: this.config.maxCacheSize,
      totalSize: entries.reduce((sum, entry) => sum + entry.data.byteLength, 0),
      hitRate: this.metrics.cacheHits / (this.metrics.cacheHits + this.metrics.cacheMisses) || 0,
      averageHitCount: entries.reduce((sum, entry) => sum + entry.metadata.hitCount, 0) / entries.length || 0
    };
  }

  /**
   * Clear all cached data
   */
  clearCache(): void {
    this.cache.clear();
    console.log('[DataFetchingService] Cache cleared');
  }

  /**
   * Update average latency metric
   */
  private updateLatencyMetric(latency: number): void {
    const totalLatency = this.metrics.averageLatency * this.metrics.successfulRequests + latency;
    this.metrics.averageLatency = totalLatency / (this.metrics.successfulRequests);
  }

  /**
   * Utility delay function
   */
  private delay(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * Cleanup resources
   */
  destroy(): void {
    if (this.backgroundIntervalId) {
      clearInterval(this.backgroundIntervalId);
    }
    
    if (this.websocket) {
      this.websocket.close();
    }
    
    this.cache.clear();
    this.activeRequests.clear();
    this.requestQueue.length = 0;
    this.subscribers.clear();
    
    console.log('[DataFetchingService] Service destroyed');
  }
}