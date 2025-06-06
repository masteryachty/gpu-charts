import { Page } from '@playwright/test';

export interface DataColumn {
  name: string;
  record_size: number;
  num_records: number;
  data_length: number;
}

export interface MockDataOptions {
  symbol?: string;
  type?: string;
  startTime?: number;
  endTime?: number;
  columns?: string[];
  recordCount?: number;
}

export class DataMockHelper {
  constructor(private page: Page) {}

  /**
   * Generate realistic binary time series data
   */
  generateTimeSeries(startTime: number, endTime: number, intervalSeconds: number = 60): Float32Array {
    const duration = endTime - startTime;
    const count = Math.floor(duration / intervalSeconds);
    const times = new Float32Array(count);
    
    for (let i = 0; i < count; i++) {
      times[i] = startTime + (i * intervalSeconds);
    }
    
    return times;
  }

  /**
   * Generate realistic price data with trends and volatility
   */
  generatePriceData(count: number, basePrice: number = 50000, volatility: number = 0.02): Float32Array {
    const prices = new Float32Array(count);
    let currentPrice = basePrice;
    
    for (let i = 0; i < count; i++) {
      // Add trend (slight upward bias)
      const trend = Math.random() * 0.001 - 0.0005;
      
      // Add volatility
      const change = (Math.random() - 0.5) * volatility;
      
      currentPrice *= (1 + trend + change);
      prices[i] = Math.max(currentPrice, 1); // Prevent negative prices
    }
    
    return prices;
  }

  /**
   * Generate realistic volume data
   */
  generateVolumeData(count: number, avgVolume: number = 1000000): Float32Array {
    const volumes = new Float32Array(count);
    
    for (let i = 0; i < count; i++) {
      // Volume follows log-normal distribution
      const randomFactor = Math.exp(Math.random() * 2 - 1);
      volumes[i] = avgVolume * randomFactor;
    }
    
    return volumes;
  }

  /**
   * Create a complete mock data response matching your API format
   */
  createMockDataResponse(options: MockDataOptions = {}): string {
    const {
      symbol = 'BTC',
      type = 'MD',
      startTime = 1745322750,
      endTime = 1745326350,
      columns = ['time', 'price', 'volume'],
      recordCount
    } = options;

    const duration = endTime - startTime;
    const count = recordCount || Math.min(Math.floor(duration / 60), 10000); // 1 minute intervals, max 10k

    const columnMetas: DataColumn[] = columns.map(col => ({
      name: col,
      record_size: 4,
      num_records: count,
      data_length: count * 4
    }));

    const header = { columns: columnMetas };
    return JSON.stringify(header) + '\n';
  }

  /**
   * Create mock binary data buffer (for full binary response testing)
   */
  createMockBinaryData(columns: string[], recordCount: number): ArrayBuffer {
    const totalSize = columns.length * recordCount * 4;
    const buffer = new ArrayBuffer(totalSize);
    const view = new Float32Array(buffer);

    let offset = 0;
    
    for (const column of columns) {
      let data: Float32Array;
      
      switch (column) {
        case 'time':
          data = this.generateTimeSeries(1745322750, 1745322750 + recordCount * 60);
          break;
        case 'price':
          data = this.generatePriceData(recordCount);
          break;
        case 'volume':
          data = this.generateVolumeData(recordCount);
          break;
        default:
          // Generate random data for unknown columns
          data = new Float32Array(recordCount);
          for (let i = 0; i < recordCount; i++) {
            data[i] = Math.random() * 1000;
          }
      }
      
      view.set(data, offset);
      offset += recordCount;
    }

    return buffer;
  }

  /**
   * Mock the data API endpoint with realistic responses
   */
  async mockDataAPI(options: MockDataOptions = {}) {
    await this.page.route('**/api/data*', route => {
      const url = new URL(route.request().url());
      const symbol = url.searchParams.get('symbol') || options.symbol || 'BTC';
      const type = url.searchParams.get('type') || options.type || 'MD';
      const start = parseInt(url.searchParams.get('start') || '1745322750');
      const end = parseInt(url.searchParams.get('end') || '1745326350');
      const columns = url.searchParams.get('columns')?.split(',') || ['time', 'price'];

      // Validate parameters
      if (!symbol || !type || start >= end) {
        route.fulfill({
          status: 400,
          body: 'Invalid parameters'
        });
        return;
      }

      const mockResponse = this.createMockDataResponse({
        symbol,
        type,
        startTime: start,
        endTime: end,
        columns,
        recordCount: options.recordCount
      });

      route.fulfill({
        status: 200,
        contentType: 'application/octet-stream',
        body: mockResponse
      });
    });
  }

  /**
   * Mock the symbols API endpoint
   */
  async mockSymbolsAPI(symbols: string[] = ['BTC', 'ETH', 'AAPL', 'TSLA', 'GOOGL']) {
    await this.page.route('**/api/symbols', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ symbols })
      });
    });
  }

  /**
   * Mock API errors for testing error handling
   */
  async mockAPIErrors(errorType: 'network' | 'server' | 'timeout' | 'malformed') {
    await this.page.route('**/api/data*', route => {
      switch (errorType) {
        case 'network':
          route.abort('failed');
          break;
        case 'server':
          route.fulfill({
            status: 500,
            body: 'Internal server error'
          });
          break;
        case 'timeout':
          // Don't respond to simulate timeout
          break;
        case 'malformed':
          route.fulfill({
            status: 200,
            contentType: 'application/octet-stream',
            body: 'invalid json response'
          });
          break;
      }
    });
  }

  /**
   * Create realistic multi-day data scenario
   */
  async mockMultiDayData(startTime: number, endTime: number) {
    await this.page.route('**/api/data*', route => {
      const url = new URL(route.request().url());
      const start = parseInt(url.searchParams.get('start') || startTime.toString());
      const end = parseInt(url.searchParams.get('end') || endTime.toString());
      const columns = url.searchParams.get('columns')?.split(',') || ['time', 'price'];

      // Calculate realistic record count for multi-day range
      const durationHours = (end - start) / 3600;
      const recordCount = Math.min(Math.floor(durationHours * 60), 100000); // 1 per minute, max 100k

      const response = this.createMockDataResponse({
        startTime: start,
        endTime: end,
        columns,
        recordCount
      });

      // Add delay to simulate realistic multi-day data loading
      setTimeout(() => {
        route.fulfill({
          status: 200,
          contentType: 'application/octet-stream',
          body: response
        });
      }, 100);
    });
  }

  /**
   * Mock slow data loading for performance testing
   */
  async mockSlowDataAPI(delayMs: number = 2000) {
    await this.page.route('**/api/data*', route => {
      const url = new URL(route.request().url());
      const columns = url.searchParams.get('columns')?.split(',') || ['time', 'price'];
      
      const response = this.createMockDataResponse({
        columns,
        recordCount: 5000 // Large dataset
      });

      setTimeout(() => {
        route.fulfill({
          status: 200,
          contentType: 'application/octet-stream',
          body: response
        });
      }, delayMs);
    });
  }
}

/**
 * Test data constants for common scenarios
 */
export const TestScenarios = {
  SMALL_DATASET: { recordCount: 100, duration: 6000 },      // 100 minutes
  MEDIUM_DATASET: { recordCount: 1000, duration: 60000 },    // ~17 hours  
  LARGE_DATASET: { recordCount: 10000, duration: 600000 },   // ~7 days
  HUGE_DATASET: { recordCount: 50000, duration: 3000000 },   // ~35 days

  TIME_RANGES: {
    MINUTES: { start: 1745322750, end: 1745322990 },     // 4 minutes
    HOUR: { start: 1745322750, end: 1745326350 },        // 1 hour
    DAY: { start: 1745322750, end: 1745409150 },         // 24 hours  
    WEEK: { start: 1745322750, end: 1746927550 },        // 7 days
    MONTH: { start: 1745322750, end: 1747914750 }        // 30 days
  },

  SYMBOLS: {
    CRYPTO: ['BTC', 'ETH', 'ADA', 'DOT', 'LINK'],
    STOCKS: ['AAPL', 'TSLA', 'GOOGL', 'MSFT', 'AMZN'],
    // cSpell:ignore EURUSD GBPUSD USDJPY AUDUSD
    FOREX: ['EURUSD', 'GBPUSD', 'USDJPY', 'AUDUSD'],
    COMMODITIES: ['GOLD', 'SILVER', 'OIL', 'WHEAT']
  },

  COLUMNS: {
    BASIC: ['time', 'price'],
    WITH_VOLUME: ['time', 'price', 'volume'],
    // cSpell:ignore OHLC
    OHLC: ['time', 'open', 'high', 'low', 'close'],
    FULL: ['time', 'open', 'high', 'low', 'close', 'volume']
  }
};