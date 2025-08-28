import { http, HttpResponse } from 'msw';

// Mock API responses for testing
export const handlers = [
  // Mock data API endpoint
  http.get('*/api/data', ({ request }) => {
    const url = new URL(request.url);
    const symbol = url.searchParams.get('symbol') || 'BTC-USD';
    const start = parseInt(url.searchParams.get('start') || '0');
    const end = parseInt(url.searchParams.get('end') || '0');
    const columns = url.searchParams.get('columns') || 'time,price';

    // Generate mock binary data
    const mockData = generateMockData(symbol, start, end, columns.split(','));
    
    return HttpResponse.json({
      metadata: {
        symbol,
        start,
        end,
        columns: columns.split(','),
        count: mockData.length,
        format: 'binary'
      },
      data: mockData
    });
  }),

  // Mock symbols API endpoint
  http.get('*/api/symbols', () => {
    return HttpResponse.json({
      symbols: [
        'BTC-USD',
        'ETH-USD', 
        'ADA-USD',
        'DOT-USD',
        'LINK-USD',
        'AVAX-USD',
        'SOL-USD',
        'MATIC-USD'
      ]
    });
  }),

  // Mock exchange data
  http.get('*/api/exchanges', ({ request }) => {
    const url = new URL(request.url);
    const symbol = url.searchParams.get('symbol') || 'BTC-USD';
    
    return HttpResponse.json({
      exchanges: [
        {
          name: 'coinbase',
          displayName: 'Coinbase',
          symbols: ['BTC-USD', 'ETH-USD', 'ADA-USD']
        },
        {
          name: 'kraken', 
          displayName: 'Kraken',
          symbols: ['BTC-USD', 'ETH-USD', 'DOT-USD']
        },
        {
          name: 'binance',
          displayName: 'Binance',
          symbols: ['BTC-USD', 'ETH-USD', 'LINK-USD', 'AVAX-USD']
        }
      ]
    });
  }),

  // Catch-all handler for unhandled requests
  http.all('*', ({ request }) => {
    console.warn(`Unhandled ${request.method} request to ${request.url}`);
    return new HttpResponse(null, { status: 404 });
  })
];

// Helper function to generate mock time-series data
function generateMockData(symbol: string, start: number, end: number, columns: string[]) {
  const data: Array<{ [key: string]: number }> = [];
  const timeRange = end - start;
  const pointCount = Math.min(1000, Math.max(10, timeRange / 60)); // 1 point per minute, max 1000 points
  
  let basePrice = getBasePrice(symbol);
  
  for (let i = 0; i < pointCount; i++) {
    const time = start + (i * timeRange) / (pointCount - 1);
    const point: { [key: string]: number } = {};
    
    columns.forEach(column => {
      switch (column) {
        case 'time':
          point.time = time;
          break;
        case 'price':
        case 'best_bid':
        case 'best_ask':
          // Simulate price movement with random walk
          const volatility = 0.02;
          const change = (Math.random() - 0.5) * volatility;
          basePrice *= (1 + change);
          
          if (column === 'best_bid') {
            point.best_bid = basePrice * 0.9995; // Slightly below price
          } else if (column === 'best_ask') {
            point.best_ask = basePrice * 1.0005; // Slightly above price
          } else {
            point.price = basePrice;
          }
          break;
        case 'volume':
          point.volume = Math.random() * 10000 + 1000;
          break;
        case 'side':
          point.side = Math.random() > 0.5 ? 1 : 0; // 1 for buy, 0 for sell
          break;
        default:
          point[column] = Math.random() * 100;
      }
    });
    
    data.push(point);
  }
  
  return data;
}

function getBasePrice(symbol: string): number {
  const prices: { [key: string]: number } = {
    'BTC-USD': 45000,
    'ETH-USD': 3000,
    'ADA-USD': 0.45,
    'DOT-USD': 18,
    'LINK-USD': 15,
    'AVAX-USD': 35,
    'SOL-USD': 95,
    'MATIC-USD': 0.85
  };
  
  return prices[symbol] || 100;
}