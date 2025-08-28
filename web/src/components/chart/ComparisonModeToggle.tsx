import { useEffect, useState } from 'react';
import { formatExchangeName, getExchangeColor, getAvailableExchanges, type ExchangeSymbol } from '../../services/symbolApi';
import { ExchangeIndicator } from '../common/AccessibleIndicator';

interface ComparisonModeToggleProps {
  comparisonMode: boolean;
  onComparisonModeChange: (enabled: boolean) => void;
  selectedExchanges: string[];
  onSelectedExchangesChange: (exchanges: string[]) => void;
  onToggleExchange: (exchange: string, symbol: string) => void;
  onSymbolChange: (symbol: string) => void;
  currentExchange: string;
  currentSymbolWithoutExchange: string;
  normalizedBaseSymbol: string;
}

export default function ComparisonModeToggle({
  comparisonMode,
  onComparisonModeChange,
  selectedExchanges,
  onToggleExchange,
  onSymbolChange,
  currentExchange,
  currentSymbolWithoutExchange,
  normalizedBaseSymbol
}: ComparisonModeToggleProps) {
  const [availableExchanges, setAvailableExchanges] = useState<ExchangeSymbol[]>([]);
  const [loadingExchanges, setLoadingExchanges] = useState(false);

  // Fetch available exchanges when normalized symbol changes
  useEffect(() => {
    const fetchExchanges = async () => {
      if (!normalizedBaseSymbol) return;
      
      setLoadingExchanges(true);
      try {
        const exchanges = await getAvailableExchanges(normalizedBaseSymbol);
        setAvailableExchanges(exchanges);
      } catch (error) {
        console.error('[ComparisonModeToggle] Failed to fetch exchanges:', error);
        setAvailableExchanges([]);
      } finally {
        setLoadingExchanges(false);
      }
    };

    fetchExchanges();
  }, [normalizedBaseSymbol]);

  const handleComparisonModeToggle = () => {
    const newComparisonMode = !comparisonMode;
    onComparisonModeChange(newComparisonMode);
    
    // If turning on comparison mode, initialize with current selection
    if (newComparisonMode && availableExchanges.length > 0) {
      // Find the current exchange-symbol in available exchanges
      const currentMatch = availableExchanges.find(
        es => es.exchange === currentExchange && es.symbol === currentSymbolWithoutExchange
      );
      
      if (currentMatch) {
        const exchangeSymbolId = `${currentMatch.exchange}:${currentMatch.symbol}`;
        // This would need to be handled by parent component
        // onSelectedExchangesChange([exchangeSymbolId]);
      } else if (availableExchanges.length > 0) {
        // Fallback to first available exchange-symbol
        const firstExchange = availableExchanges[0];
        const exchangeSymbolId = `${firstExchange.exchange}:${firstExchange.symbol}`;
        // onSelectedExchangesChange([exchangeSymbolId]);
      }
    }
  };

  return (
    <div className="space-y-2" role="group" aria-labelledby="comparison-mode-label">
      <div className="flex items-center justify-between">
        <label id="comparison-mode-label" className="text-gray-300 text-sm font-medium">
          Comparison Mode
        </label>
        <button
          onClick={handleComparisonModeToggle}
          data-testid="comparison-toggle"
          role="switch"
          aria-checked={comparisonMode}
          aria-labelledby="comparison-mode-label"
          aria-describedby="comparison-mode-description"
          className={`
            relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-gray-800
            ${comparisonMode ? 'bg-blue-600' : 'bg-gray-600'}
          `}
        >
          <span className="sr-only">
            {comparisonMode ? 'Disable' : 'Enable'} comparison mode
          </span>
          <span
            className={`
              inline-block h-4 w-4 transform rounded-full bg-white transition-transform
              ${comparisonMode ? 'translate-x-6' : 'translate-x-1'}
            `}
            aria-hidden="true"
          />
        </button>
      </div>
      <div id="comparison-mode-description" className="sr-only">
        Toggle between single exchange view and multi-exchange comparison view
      </div>
      
      {comparisonMode && (
        <div className="space-y-1">
          <p id="exchange-selection-instructions" className="text-xs text-gray-400">
            Select up to 2 exchanges to compare
          </p>
          
          {loadingExchanges ? (
            <div className="text-center py-4" role="status" aria-live="polite">
              <div className="animate-spin text-blue-500" aria-hidden="true">⚡</div>
              <p className="text-gray-400 text-sm mt-1">Loading exchanges...</p>
            </div>
          ) : availableExchanges.length === 0 ? (
            <p className="text-gray-500 text-sm py-2" role="status">No exchanges available for this symbol</p>
          ) : (
            <div 
              className="grid gap-2 max-h-48 overflow-y-auto"
              role="group"
              aria-labelledby="exchange-selection-instructions"
            >
              {availableExchanges.map((exchangeSymbol) => {
                const exchange = exchangeSymbol.exchange;
                const exchangeSymbolId = `${exchange}:${exchangeSymbol.symbol}`;
                const isSelected = selectedExchanges.includes(exchangeSymbolId);
                const isActive = exchange === currentExchange && exchangeSymbol.symbol === currentSymbolWithoutExchange;

                return (
                  <button
                    key={exchangeSymbolId}
                    data-testid={`exchange-${exchange}`}
                    onClick={() => {
                      if (comparisonMode) {
                        // In comparison mode, toggle the specific exchange-symbol combination
                        onToggleExchange(exchange, exchangeSymbol.symbol);
                      } else {
                        // In single mode, switch to this exchange
                        const newSymbol = `${exchange}:${exchangeSymbol.symbol}`;
                        onSymbolChange(newSymbol);
                        onToggleExchange(exchange, exchangeSymbol.symbol);
                        
                        // Update URL
                        const urlParams = new URLSearchParams(window.location.search);
                        urlParams.set('topic', newSymbol);
                        const newUrl = `${window.location.pathname}?${urlParams.toString()}`;
                        window.history.pushState({}, '', newUrl);
                      }
                    }}
                    className={`
                      relative px-3 py-2.5 text-sm font-medium rounded-lg
                      transition-all duration-200 transform focus:outline-none focus:ring-2 focus:ring-blue-500
                      ${(isActive || (comparisonMode && isSelected))
                        ? 'bg-gray-700 text-white shadow-lg scale-[1.02] ring-2 ring-blue-400' 
                        : 'bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200'
                      }
                      border ${(isActive || (comparisonMode && isSelected)) ? 'border-gray-500' : 'border-gray-700'}
                      hover:scale-[1.02] active:scale-[0.98]
                    `}
                    role="checkbox"
                    aria-checked={comparisonMode ? isSelected : isActive}
                    aria-label={`${formatExchangeName(exchange)} ${exchangeSymbol.symbol} ${comparisonMode ? (isSelected ? '(selected)' : '(not selected)') : (isActive ? '(current)' : '')}`}
                  >
                    <div className="flex items-center justify-between w-full">
                      <div className="flex items-center gap-2">
                        <ExchangeIndicator 
                          exchange={exchange}
                          active={isActive || (comparisonMode && isSelected)}
                          size="sm"
                        />
                        <div className="flex flex-col items-start">
                          <span className="relative z-10">{formatExchangeName(exchange)}</span>
                          <span className="text-xs text-gray-500 mt-0.5">{exchangeSymbol.symbol}</span>
                        </div>
                      </div>
                      {comparisonMode && (
                        <div className="flex items-center gap-1">
                          <span className="text-xs text-gray-400">
                            {isSelected ? 'Selected' : 'Select'}
                          </span>
                          <div 
                            className={`
                              w-4 h-4 rounded border-2 flex items-center justify-center flex-shrink-0
                              ${isSelected ? 'bg-blue-600 border-blue-600' : 'border-gray-500'}
                            `}
                            aria-hidden="true"
                          >
                            {isSelected && (
                              <svg className="w-3 h-3 text-white" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
                                <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                              </svg>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
}