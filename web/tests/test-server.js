#!/usr/bin/env node

/**
 * Test Data Server
 * 
 * Provides mock data endpoints for testing the graph visualization app
 * without requiring the full production data server.
 */

import http from 'http';
import https from 'https';
import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

class TestDataServer {
  constructor(options = {}) {
    this.port = options.port || 8443;
    this.host = options.host || 'localhost';
    this.useHttps = options.useHttps !== false; // Default to HTTPS to match production
    this.server = null;
    this.sslOptions = null;
    
    // Mock data cache
    this.dataCache = new Map();
    this.symbolsCache = ['BTC-USD', 'ETH-USD', 'ADA-USD', 'DOT-USD', 'AAPL', 'TSLA', 'GOOGL'];
    
    this.setupSSL();
  }
  
  setupSSL() {
    if (!this.useHttps) return;
    
    try {
      // Try to load existing SSL certificates
      const certPath = path.join(__dirname, '../../certs');
      const certFile = path.join(certPath, 'localhost.crt');
      const keyFile = path.join(certPath, 'localhost.key');
      
      if (fs.existsSync(certFile) && fs.existsSync(keyFile)) {
        this.sslOptions = {
          cert: fs.readFileSync(certFile),
          key: fs.readFileSync(keyFile)
        };
        console.log('[TestServer] Using existing SSL certificates');
      } else {
        console.log('[TestServer] SSL certificates not found, generating self-signed certificates');
        this.generateSelfSignedCert();
      }
    } catch (error) {
      console.warn('[TestServer] SSL setup failed, falling back to HTTP:', error.message);
      this.useHttps = false;
    }
  }
  
  generateSelfSignedCert() {
    // For testing, we'll just disable HTTPS if certs aren't available
    // In a real setup, you'd generate self-signed certificates here
    console.log('[TestServer] Self-signed certificate generation not implemented, using HTTP');
    this.useHttps = false;
  }
  
  generateMockData(symbol, startTime, endTime, columns = ['time', 'best_bid', 'best_ask', 'price', 'volume']) {
    const cacheKey = `${symbol}_${startTime}_${endTime}_${columns.join(',')}`;
    
    if (this.dataCache.has(cacheKey)) {
      return this.dataCache.get(cacheKey);
    }
    
    const timeRange = endTime - startTime;
    const dataPoints = Math.min(Math.max(Math.floor(timeRange / 60), 10), 10000); // 1 point per minute, max 10k points
    
    // Generate realistic market data
    const basePrice = this.getBasePrice(symbol);
    const volatility = this.getVolatility(symbol);
    
    const records = [];
    let currentPrice = basePrice;
    
    for (let i = 0; i < dataPoints; i++) {
      const timestamp = startTime + Math.floor((i / dataPoints) * timeRange);
      
      // Random walk with mean reversion
      const change = (Math.random() - 0.5) * volatility * currentPrice * 0.01;
      const meanReversion = (basePrice - currentPrice) * 0.001;
      currentPrice += change + meanReversion;
      
      const spread = currentPrice * 0.001; // 0.1% spread
      const volume = Math.random() * 1000 + 100;
      
      const record = {};
      columns.forEach(col => {
        switch (col) {
          case 'time':
            record[col] = timestamp;
            break;
          case 'best_bid':
            record[col] = currentPrice - spread / 2;
            break;
          case 'best_ask':
            record[col] = currentPrice + spread / 2;
            break;
          case 'price':
            record[col] = currentPrice;
            break;
          case 'volume':
            record[col] = volume;
            break;
          case 'side':
            record[col] = Math.random() > 0.5 ? 1 : 0; // Buy = 1, Sell = 0
            break;
          default:
            record[col] = Math.random() * 100;
        }
      });
      
      records.push(record);
    }
    
    // Create binary data format to match production server
    const binaryData = this.createBinaryData(records, columns);
    const response = {
      columns: columns.map(col => ({
        name: col,
        record_size: 4,
        num_records: dataPoints,
        data_length: dataPoints * 4
      })),
      records: dataPoints,
      data: binaryData
    };
    
    this.dataCache.set(cacheKey, response);
    return response;
  }
  
  getBasePrice(symbol) {
    const prices = {
      'BTC-USD': 45000,
      'ETH-USD': 3000,
      'ADA-USD': 0.5,
      'DOT-USD': 25,
      'AAPL': 180,
      'TSLA': 250,
      'GOOGL': 140
    };
    return prices[symbol] || 100;
  }
  
  getVolatility(symbol) {
    const volatilities = {
      'BTC-USD': 3.0,
      'ETH-USD': 4.0,
      'ADA-USD': 5.0,
      'DOT-USD': 4.5,
      'AAPL': 1.5,
      'TSLA': 3.5,
      'GOOGL': 1.8
    };
    return volatilities[symbol] || 2.0;
  }
  
  createBinaryData(records, columns) {
    // Create a simple binary format for testing
    const buffer = Buffer.alloc(records.length * columns.length * 4);
    let offset = 0;
    
    records.forEach(record => {
      columns.forEach(col => {
        buffer.writeFloatLE(record[col] || 0, offset);
        offset += 4;
      });
    });
    
    return buffer;
  }
  
  handleRequest(req, res) {
    // Enable CORS for all requests
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');
    
    if (req.method === 'OPTIONS') {
      res.writeHead(200);
      res.end();
      return;
    }
    
    const url = new URL(req.url, `${this.useHttps ? 'https' : 'http'}://${req.headers.host}`);
    const pathname = url.pathname;
    
    console.log(`[TestServer] ${req.method} ${pathname} ${url.search}`);
    
    try {
      if (pathname === '/api/symbols') {
        this.handleSymbols(req, res, url);
      } else if (pathname === '/api/data') {
        this.handleData(req, res, url);
      } else if (pathname === '/health') {
        this.handleHealth(req, res);
      } else {
        res.writeHead(404, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Not found' }));
      }
    } catch (error) {
      console.error('[TestServer] Error handling request:', error);
      res.writeHead(500, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ error: 'Internal server error' }));
    }
  }
  
  handleSymbols(req, res, url) {
    const response = {
      symbols: this.symbolsCache
    };
    
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(response));
  }
  
  handleData(req, res, url) {
    const params = url.searchParams;
    const symbol = params.get('symbol') || 'BTC-USD';
    const type = params.get('type') || 'MD';
    const startTime = parseInt(params.get('start')) || Math.floor(Date.now() / 1000) - 3600;
    const endTime = parseInt(params.get('end')) || Math.floor(Date.now() / 1000);
    const columnsParam = params.get('columns') || 'time,best_bid,best_ask,price,volume';
    const columns = columnsParam.split(',').map(c => c.trim());
    
    // Simulate some processing delay
    setTimeout(() => {
      const mockData = this.generateMockData(symbol, startTime, endTime, columns);
      
      // Send binary response similar to production server
      res.writeHead(200, { 
        'Content-Type': 'application/octet-stream',
        'X-Data-Columns': JSON.stringify(mockData.columns),
        'X-Data-Records': mockData.records.toString()
      });
      
      // Send header as JSON followed by binary data
      const header = JSON.stringify(mockData.columns) + '\n';
      res.write(header);
      res.end(mockData.data);
    }, Math.random() * 100 + 50); // 50-150ms delay
  }
  
  handleHealth(req, res) {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ 
      status: 'ok', 
      server: 'test-data-server',
      timestamp: Date.now(),
      uptime: process.uptime()
    }));
  }
  
  start() {
    return new Promise((resolve, reject) => {
      const requestHandler = (req, res) => this.handleRequest(req, res);
      
      if (this.useHttps && this.sslOptions) {
        this.server = https.createServer(this.sslOptions, requestHandler);
      } else {
        this.server = http.createServer(requestHandler);
        this.port = 8080; // Use HTTP port for testing
      }
      
      this.server.listen(this.port, this.host, (err) => {
        if (err) {
          reject(err);
        } else {
          const protocol = this.useHttps ? 'https' : 'http';
          console.log(`[TestServer] Server running at ${protocol}://${this.host}:${this.port}`);
          console.log(`[TestServer] Endpoints:`);
          console.log(`  - ${protocol}://${this.host}:${this.port}/api/symbols`);
          console.log(`  - ${protocol}://${this.host}:${this.port}/api/data`);
          console.log(`  - ${protocol}://${this.host}:${this.port}/health`);
          resolve();
        }
      });
      
      this.server.on('error', (err) => {
        console.error('[TestServer] Server error:', err);
        reject(err);
      });
    });
  }
  
  stop() {
    return new Promise((resolve) => {
      if (this.server) {
        this.server.close(() => {
          console.log('[TestServer] Server stopped');
          resolve();
        });
      } else {
        resolve();
      }
    });
  }
}

// CLI usage
if (import.meta.url === `file://${process.argv[1]}`) {
  const args = process.argv.slice(2);
  const options = {};
  
  for (let i = 0; i < args.length; i += 2) {
    const key = args[i]?.replace('--', '');
    const value = args[i + 1];
    
    if (key === 'port') options.port = parseInt(value);
    if (key === 'host') options.host = value;
    if (key === 'http') options.useHttps = false;
  }
  
  const server = new TestDataServer(options);
  
  process.on('SIGINT', async () => {
    console.log('\n[TestServer] Shutting down...');
    await server.stop();
    process.exit(0);
  });
  
  process.on('SIGTERM', async () => {
    console.log('\n[TestServer] Shutting down...');
    await server.stop();
    process.exit(0);
  });
  
  server.start().catch(err => {
    console.error('[TestServer] Failed to start:', err);
    process.exit(1);
  });
}

export default TestDataServer;