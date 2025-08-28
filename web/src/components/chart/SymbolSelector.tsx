import { useMemo } from 'react';

interface SymbolSelectorProps {
  symbol: string;
  onSymbolChange: (symbol: string) => void;
}

export default function SymbolSelector({ symbol, onSymbolChange }: SymbolSelectorProps) {
  // Available options (memoized to prevent dependency issues)
  const symbols = useMemo(() => ['BTC-USD', 'ETH-USD', 'ADA-USD', 'DOT-USD', 'LINK-USD', 'AVAX-USD'], []);

  return (
    <div className="space-y-2" role="group" aria-labelledby="symbol-selector-label">
      <label id="symbol-selector-label" htmlFor="symbol-selector" className="text-gray-300 text-sm font-medium">
        Trading Symbol
      </label>
      <select
        id="symbol-selector"
        data-testid="symbol-selector"
        value={symbol}
        onChange={(e) => onSymbolChange(e.target.value)}
        className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        aria-label="Select trading symbol"
        aria-describedby="symbol-selector-description"
      >
        {symbols.map(symbol => (
          <option key={symbol} value={symbol}>{symbol}</option>
        ))}
      </select>
      <div id="symbol-selector-description" className="sr-only">
        Choose the cryptocurrency trading pair to display on the chart
      </div>
    </div>
  );
}