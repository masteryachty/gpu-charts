import { Page } from '@playwright/test';

/**
 * Test Data Helper
 * 
 * Provides utilities for setting up mock data routes and responses
 * that work with the test data server
 */

export class TestDataHelper {
  constructor(private page: Page, private testServerUrl = 'http://localhost:8080') {}

  /**
   * Route all API requests to the test data server
   */
  async routeToTestServer(): Promise<void> {
    // Route symbols API
    await this.page.route('**/api/symbols', async (route) => {
      const response = await fetch(`${this.testServerUrl}/api/symbols`);
      const data = await response.json();
      
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(data)
      });
    });

    // Route data API
    await this.page.route('**/api/data**', async (route) => {
      const url = new URL(route.request().url());
      const testServerUrl = `${this.testServerUrl}/api/data${url.search}`;
      
      try {
        const response = await fetch(testServerUrl);
        const buffer = await response.arrayBuffer();
        
        await route.fulfill({
          status: response.status,
          headers: Object.fromEntries(response.headers.entries()),
          body: Buffer.from(buffer)
        });
      } catch (error) {
        console.error('Failed to fetch from test server:', error);
        await route.fulfill({
          status: 500,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Test server error' })
        });
      }
    });
  }

  /**
   * Mock successful symbol response
   */
  async mockSymbols(symbols: string[] = ['BTC-USD', 'ETH-USD', 'ADA-USD']): Promise<void> {
    await this.page.route('**/api/symbols', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ symbols })
      });
    });
  }

  /**
   * Mock data API response with generated data
   */
  async mockDataResponse(options: {
    symbol?: string;
    records?: number;
    columns?: string[];
    delay?: number;
  } = {}): Promise<void> {
    const {
      symbol = 'BTC-USD',
      records = 1000,
      columns = ['time', 'best_bid', 'best_ask', 'price', 'volume'],
      delay = 0
    } = options;

    await this.page.route('**/api/data**', async (route) => {
      if (delay > 0) {
        await new Promise(resolve => setTimeout(resolve, delay));
      }

      // Generate mock binary data
      const mockData = this.generateMockBinaryData(records, columns);
      
      await route.fulfill({
        status: 200,
        contentType: 'application/octet-stream',
        headers: {
          'X-Data-Columns': JSON.stringify(columns.map(col => ({
            name: col,
            record_size: 4,
            num_records: records,
            data_length: records * 4
          }))),
          'X-Data-Records': records.toString()
        },
        body: mockData
      });
    });
  }

  /**
   * Mock API error responses
   */
  async mockApiError(endpoint: 'symbols' | 'data' | 'both', status = 500, message = 'Server error'): Promise<void> {
    const routes = endpoint === 'both' ? ['**/api/symbols', '**/api/data**'] 
                  : [`**/api/${endpoint}${endpoint === 'data' ? '**' : ''}`];

    for (const routePattern of routes) {
      await this.page.route(routePattern, route => {
        route.fulfill({
          status,
          contentType: 'application/json',
          body: JSON.stringify({ error: message })
        });
      });
    }
  }

  /**
   * Mock slow network responses
   */
  async mockSlowResponse(endpoint: 'symbols' | 'data' | 'both', delay = 5000): Promise<void> {
    const routes = endpoint === 'both' ? ['**/api/symbols', '**/api/data**'] 
                  : [`**/api/${endpoint}${endpoint === 'data' ? '**' : ''}`];

    for (const routePattern of routes) {
      await this.page.route(routePattern, async (route) => {
        await new Promise(resolve => setTimeout(resolve, delay));
        await route.continue();
      });
    }
  }

  /**
   * Clear all API route mocks
   */
  async clearMocks(): Promise<void> {
    await this.page.unroute('**/api/symbols');
    await this.page.unroute('**/api/data**');
  }

  /**
   * Generate mock binary data for testing
   */
  private generateMockBinaryData(records: number, columns: string[]): Buffer {
    const buffer = Buffer.alloc(records * columns.length * 4);
    let offset = 0;

    const baseTime = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago
    let currentPrice = 45000; // Starting price

    for (let i = 0; i < records; i++) {
      for (const column of columns) {
        let value: number;

        switch (column) {
          case 'time':
            value = baseTime + (i * 60); // 1 minute intervals
            break;
          case 'best_bid':
            value = currentPrice - (currentPrice * 0.001); // 0.1% below price
            break;
          case 'best_ask':
            value = currentPrice + (currentPrice * 0.001); // 0.1% above price
            break;
          case 'price':
            // Random walk
            currentPrice += (Math.random() - 0.5) * currentPrice * 0.02;
            value = currentPrice;
            break;
          case 'volume':
            value = Math.random() * 1000 + 100;
            break;
          case 'side':
            value = Math.random() > 0.5 ? 1 : 0;
            break;
          default:
            value = Math.random() * 100;
        }

        buffer.writeFloatLE(value, offset);
        offset += 4;
      }
    }

    return buffer;
  }

  /**
   * Wait for API request to complete
   */
  async waitForApiRequest(endpoint: 'symbols' | 'data', timeout = 5000): Promise<void> {
    const pattern = endpoint === 'symbols' ? '**/api/symbols' : '**/api/data**';
    
    await this.page.waitForRequest(request => {
      return request.url().includes(`/api/${endpoint}`);
    }, { timeout });
  }

  /**
   * Get the current test server URL
   */
  getTestServerUrl(): string {
    return this.testServerUrl;
  }

  /**
   * Check if test server is available
   */
  async isTestServerAvailable(): Promise<boolean> {
    try {
      const response = await fetch(`${this.testServerUrl}/health`);
      return response.ok;
    } catch {
      return false;
    }
  }
}

/**
 * Factory function to create TestDataHelper
 */
export function createTestDataHelper(page: Page, testServerUrl?: string): TestDataHelper {
  return new TestDataHelper(page, testServerUrl);
}

/**
 * Common test data sets
 */
export const TEST_DATA_SETS = {
  CRYPTO_SYMBOLS: ['BTC-USD', 'ETH-USD', 'ADA-USD', 'DOT-USD'],
  STOCK_SYMBOLS: ['AAPL', 'TSLA', 'GOOGL', 'MSFT'],
  FOREX_SYMBOLS: ['EUR-USD', 'GBP-USD', 'USD-JPY'],
  
  TIME_RANGES: {
    SHORT: { start: Math.floor(Date.now() / 1000) - 3600, end: Math.floor(Date.now() / 1000) }, // 1 hour
    MEDIUM: { start: Math.floor(Date.now() / 1000) - 86400, end: Math.floor(Date.now() / 1000) }, // 1 day
    LONG: { start: Math.floor(Date.now() / 1000) - 604800, end: Math.floor(Date.now() / 1000) }, // 1 week
  },
  
  COLUMNS: {
    BASIC: ['time', 'price'],
    OHLC: ['time', 'open', 'high', 'low', 'close'],
    FULL: ['time', 'best_bid', 'best_ask', 'price', 'volume', 'side'],
    MARKET_DATA: ['time', 'best_bid', 'best_ask', 'price', 'volume']
  }
};