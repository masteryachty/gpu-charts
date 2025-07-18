# Manual Test Plan for Candlestick Chart Feature

## Test Setup
1. Start the development server: `npm run dev:suite`
2. Navigate to http://localhost:3000/app
3. Wait for the chart to load with data

## Test Cases

### 1. Chart Type Controls Display
- [ ] Verify "Line" button is visible and has blue background
- [ ] Verify "Candlestick" button is visible and has gray background
- [ ] Verify timeframe dropdown is NOT visible initially

### 2. Switch to Candlestick Chart
- [ ] Click "Candlestick" button
- [ ] Verify "Candlestick" button now has blue background
- [ ] Verify "Line" button now has gray background
- [ ] Verify chart switches from line to candlestick visualization

### 3. Timeframe Dropdown Behavior
- [ ] When candlestick is selected, verify "Timeframe:" label appears
- [ ] Verify dropdown shows with default value "1 minute"
- [ ] Verify dropdown contains options: 1 minute, 5 minutes, 15 minutes, 1 hour

### 4. Change Timeframe
- [ ] Select "5 minutes" from dropdown
- [ ] Verify candles aggregate to 5-minute periods
- [ ] Select "15 minutes" from dropdown
- [ ] Verify candles aggregate to 15-minute periods
- [ ] Select "1 hour" from dropdown
- [ ] Verify candles aggregate to 1-hour periods

### 5. Chart Type Switching
- [ ] While on candlestick with 5-minute timeframe, click "Line"
- [ ] Verify chart switches to line display
- [ ] Verify timeframe dropdown disappears
- [ ] Click "Candlestick" again
- [ ] Verify timeframe is still set to 5 minutes (persistence)

### 6. Zoom and Pan
- [ ] In candlestick mode, use mouse wheel to zoom in/out
- [ ] Verify candles resize appropriately
- [ ] Click and drag to pan
- [ ] Verify partial candles appear at edges when panning

### 7. Visual Verification
- [ ] Verify green candles for bullish (close > open)
- [ ] Verify red candles for bearish (close < open)
- [ ] Verify yellow candles for doji (close = open)
- [ ] Verify wicks extend to high/low values
- [ ] Verify candle bodies span open to close

### 8. Performance
- [ ] Rapidly switch between Line and Candlestick multiple times
- [ ] Verify no crashes or freezes
- [ ] Verify chart remains responsive

### 9. Edge Cases
- [ ] Zoom out to view very long time range
- [ ] Verify candles aggregate appropriately
- [ ] Zoom in to view very short time range
- [ ] Verify individual candles are visible

## Console Checks
Open browser DevTools console and verify:
- [ ] No error messages when switching chart types
- [ ] No error messages when changing timeframes
- [ ] Console logs show OHLC aggregation happening

## Screenshots
Take screenshots of:
1. Line chart view
2. Candlestick chart with 1-minute timeframe
3. Candlestick chart with 1-hour timeframe
4. Zoomed-in view showing candle details
5. Zoomed-out view showing many candles