# Symbol Search Integration Test Guide

## Overview
The new symbol search feature provides a powerful, professional-grade trading symbol selector with intelligent search, cross-exchange support, and beautiful UI.

## Features Implemented

### 1. Smart Autocomplete
- Real-time search with 300ms debouncing
- Fuzzy matching on multiple fields
- Relevance scoring with visual indicators

### 2. Rich Information Display
- **Symbol Details**: Normalized ID, display name, description
- **Exchange Support**: Shows all exchanges where the symbol trades
- **Category Icons**: Visual indicators for crypto, forex, commodities
- **Relevance Scores**: Color-coded match quality (Exact, High, Good, Partial)

### 3. Keyboard Navigation
- `↑/↓` - Navigate through results
- `Enter` - Select highlighted result
- `Escape` - Close dropdown
- `Tab` - Standard focus navigation

### 4. Exchange Indicators
Each result shows colored pills for supported exchanges:
- **Coinbase** (Blue): `BTC-USD`
- **Binance** (Yellow): `BTCUSDT`
- **Bitfinex** (Green): `tBTCUSD`
- **Kraken** (Purple): `XBT/USD`
- **OKX** (Black): `BTC-USDT`

### 5. Search Capabilities
Try these search queries:
- `btc` - Find Bitcoin pairs
- `bitcoin` - Search by display name
- `eth` - Find Ethereum
- `usd` - Find USD pairs
- `layer2` - Find Layer 2 solutions (ARB, OP)
- `defi` - Find DeFi tokens
- `doge` - Find meme coins

## Testing Instructions

### 1. Start the Development Stack
```bash
# Terminal 1: Start the server
npm run dev:server

# Terminal 2: Start the web app with WASM
npm run dev:web
```

### 2. Access the Application
Open: http://localhost:3000/app

### 3. Test Search Functionality

#### Basic Search
1. Click on the search box in the header
2. Type "btc" - should show Bitcoin results
3. Notice the relevance scores and exchange indicators

#### Keyboard Navigation
1. Type a search query
2. Use arrow keys to navigate
3. Press Enter to select
4. Press Escape to close

#### Exchange Information
1. Search for "bitcoin"
2. Observe the exchange pills showing where it trades
3. Note the different symbol formats per exchange

#### Category Filtering
1. Search for "layer2" - should show Arbitrum and Optimism
2. Search for "defi" - should show Uniswap
3. Notice the category icons

### 4. Performance Testing
- Type rapidly to test debouncing
- Search cache prevents redundant API calls
- Results appear within 300-500ms

## API Endpoints Used

### Symbol Search
```
GET /api/symbol-search?q={query}
```

Returns:
```json
{
  "results": [{
    "normalized_id": "BTC/USD",
    "display_name": "Bitcoin / US Dollar",
    "description": "Bitcoin to US Dollar spot trading pair",
    "base": "BTC",
    "quote": "USD",
    "category": "crypto",
    "exchanges": [
      {"exchange": "coinbase", "symbol": "BTC-USD"},
      {"exchange": "binance", "symbol": "BTCUSDT"}
    ],
    "relevance_score": 150.0
  }]
}
```

## UI Components

### SymbolSearch Component
Location: `web/src/components/SymbolSearch.tsx`
- Main search component with dropdown
- Handles all interaction logic
- Manages search state and results

### API Service
Location: `web/src/services/symbolApi.ts`
- Handles API communication
- Implements caching
- Provides utility functions

### Styling
- Professional dark theme
- Smooth animations
- Responsive design
- Accessibility features

## Customization Options

### Change Search Debounce
In `SymbolSearch.tsx`:
```typescript
const debouncedQuery = useDebounce(query, 300); // Change 300 to desired ms
```

### Modify Cache TTL
In `symbolApi.ts`:
```typescript
const CACHE_TTL = 60000; // Change to desired cache duration in ms
```

### Customize Exchange Colors
In `symbolApi.ts`, modify `getExchangeColor()` function

## Troubleshooting

### No Results Appearing
1. Check server is running: `npm run dev:server`
2. Verify API endpoint: `curl -k https://localhost:8443/api/symbol-search?q=btc`
3. Check browser console for errors

### SSL Certificate Errors
1. Regenerate certificates: `npm run setup:ssl`
2. Accept self-signed cert in browser

### Slow Performance
1. Check network tab for API response times
2. Verify debounce is working (300ms delay)
3. Check cache is functioning

## Next Steps

Potential enhancements:
1. Add favorite symbols with star icons
2. Recent searches history
3. Advanced filters (by exchange, category)
4. Real-time price display in results
5. Quick chart preview on hover
6. Multi-symbol comparison
7. Watchlist integration
8. Hotkeys for quick symbol switching