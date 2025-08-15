/**
 * Simple test server for providing mock data during testing
 * This server is used by Playwright tests to provide consistent test data
 */

import http from 'http';
import https from 'https';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const PORT = process.argv.includes('--port') 
  ? parseInt(process.argv[process.argv.indexOf('--port') + 1]) 
  : 8080;

const USE_HTTPS = !process.argv.includes('--http');

// Mock data for testing
const mockData = {
  'BTC-USD': {
    symbol: 'BTC-USD',
    data: generateMockData('BTC-USD', 50000, 60000)
  },
  'ETH-USD': {
    symbol: 'ETH-USD',
    data: generateMockData('ETH-USD', 3000, 4000)
  },
  'ADA-USD': {
    symbol: 'ADA-USD',
    data: generateMockData('ADA-USD', 0.5, 1.5)
  }
};

function generateMockData(symbol, minPrice, maxPrice) {
  const data = [];
  const numPoints = 1000;
  const startTime = 1745322750;
  const interval = 60; // 1 minute intervals
  
  for (let i = 0; i < numPoints; i++) {
    const time = startTime + (i * interval);
    const price = minPrice + Math.random() * (maxPrice - minPrice);
    const bid = price - (price * 0.001); // 0.1% spread
    const ask = price + (price * 0.001);
    const volume = Math.random() * 1000;
    
    data.push({
      time,
      price,
      best_bid: bid,
      best_ask: ask,
      volume,
      side: Math.random() > 0.5 ? 1 : 0
    });
  }
  
  return data;
}

function handleRequest(req, res) {
  const parsedUrl = new URL(req.url, `http://localhost:${PORT}`);
  const pathname = parsedUrl.pathname;
  
  // CORS headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  
  if (req.method === 'OPTIONS') {
    res.writeHead(200);
    res.end();
    return;
  }
  
  // Health check endpoint
  if (pathname === '/health') {
    res.writeHead(200, { 'Content-Type': 'text/plain' });
    res.end('OK');
    return;
  }
  
  // API endpoints
  if (pathname === '/api/symbols') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      symbols: Object.keys(mockData)
    }));
    return;
  }
  
  if (pathname === '/api/data') {
    const query = Object.fromEntries(parsedUrl.searchParams);
    const symbol = query.symbol || 'BTC-USD';
    const start = parseInt(query.start) || 1745322750;
    const end = parseInt(query.end) || 1745409150;
    const columns = (query.columns || 'time,best_bid,best_ask').split(',');
    
    if (!mockData[symbol]) {
      res.writeHead(404, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Symbol not found' }));
      return;
    }
    
    // Filter data by time range
    const filteredData = mockData[symbol].data.filter(
      d => d.time >= start && d.time <= end
    );
    
    // Create binary response (simplified - normally would be proper binary)
    const response = {
      metadata: {
        symbol,
        start,
        end,
        columns,
        count: filteredData.length
      },
      data: filteredData.map(d => {
        const row = {};
        columns.forEach(col => {
          row[col] = d[col] || 0;
        });
        return row;
      })
    };
    
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(response));
    return;
  }
  
  // Default 404
  res.writeHead(404, { 'Content-Type': 'text/plain' });
  res.end('Not Found');
}

// Create server
let server;

if (USE_HTTPS) {
  // Try to use SSL certificates if available
  try {
    const options = {
      key: fs.readFileSync(path.join(__dirname, '../../certs/localhost.key')),
      cert: fs.readFileSync(path.join(__dirname, '../../certs/localhost.crt'))
    };
    server = https.createServer(options, handleRequest);
  } catch (e) {
    console.log('SSL certificates not found, falling back to HTTP');
    server = http.createServer(handleRequest);
  }
} else {
  server = http.createServer(handleRequest);
}

server.listen(PORT, () => {
  console.log(`Test server running on ${USE_HTTPS ? 'https' : 'http'}://localhost:${PORT}`);
  console.log('Available endpoints:');
  console.log(`  /health - Health check`);
  console.log(`  /api/symbols - List available symbols`);
  console.log(`  /api/data - Get time series data`);
});

// Graceful shutdown
process.on('SIGTERM', () => {
  server.close(() => {
    console.log('Test server stopped');
  });
});

process.on('SIGINT', () => {
  server.close(() => {
    console.log('Test server stopped');
  });
});