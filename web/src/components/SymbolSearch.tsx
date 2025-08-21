import { useState, useEffect, useRef, useCallback, KeyboardEvent } from 'react';
import { Search, ChevronDown, TrendingUp, Layers, DollarSign, Hash, X } from 'lucide-react';
import { useDebounce } from '../hooks/useDebounce';
import { 
  searchSymbols, 
  SearchResult, 
  formatExchangeName, 
  getExchangeColor,
  getRelevanceIndicator 
} from '../services/symbolApi';
import { useAppStore } from '../store/useAppStore';
import clsx from 'clsx';

interface SymbolSearchProps {
  className?: string;
  placeholder?: string;
  onSymbolSelect?: (symbol: string) => void;
}

export default function SymbolSearch({ 
  className, 
  placeholder = "Search symbols... (e.g. 'btc usd' or 'eth/usdt')",
  onSymbolSelect 
}: SymbolSearchProps) {
  const { currentSymbol, setCurrentSymbol } = useAppStore();
  const [query, setQuery] = useState('');
  const [isOpen, setIsOpen] = useState(false);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const resultsRef = useRef<HTMLDivElement>(null);
  
  const debouncedQuery = useDebounce(query, 300);

  // Perform search when debounced query changes
  useEffect(() => {
    if (debouncedQuery.trim()) {
      performSearch(debouncedQuery);
    } else {
      setResults([]);
      setError(null);
    }
  }, [debouncedQuery]);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const performSearch = async (searchQuery: string) => {
    setIsLoading(true);
    setError(null);
    
    try {
      const searchResults = await searchSymbols(searchQuery);
      setResults(searchResults);
      setSelectedIndex(-1);
    } catch (err) {
      setError('Failed to search symbols');
      console.error('Search error:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSelectSymbol = useCallback((result: SearchResult, exchangeSymbol?: string) => {
    // Use specific exchange symbol if provided, otherwise first exchange
    const symbolToUse = exchangeSymbol || result.exchanges[0]?.symbol;
    if (symbolToUse) {
      setCurrentSymbol(symbolToUse);
      onSymbolSelect?.(symbolToUse);
      
      // Update URL with new symbol
      const urlParams = new URLSearchParams(window.location.search);
      urlParams.set('topic', symbolToUse);
      const newUrl = `${window.location.pathname}?${urlParams.toString()}`;
      window.history.pushState({}, '', newUrl);
      
      setQuery('');
      setIsOpen(false);
      setResults([]);
    }
  }, [setCurrentSymbol, onSymbolSelect]);

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (!isOpen && e.key === 'ArrowDown') {
      setIsOpen(true);
      return;
    }

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex(prev => 
          prev < results.length - 1 ? prev + 1 : prev
        );
        break;
        
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex(prev => prev > -1 ? prev - 1 : -1);
        break;
        
      case 'Enter':
        e.preventDefault();
        if (selectedIndex >= 0 && selectedIndex < results.length) {
          handleSelectSymbol(results[selectedIndex], undefined);
        }
        break;
        
      case 'Escape':
        e.preventDefault();
        setIsOpen(false);
        setQuery('');
        break;
    }
  };

  // Scroll selected item into view
  useEffect(() => {
    if (selectedIndex >= 0 && resultsRef.current) {
      const items = resultsRef.current.querySelectorAll('[data-result-item]');
      if (items[selectedIndex]) {
        items[selectedIndex].scrollIntoView({
          block: 'nearest',
          behavior: 'smooth'
        });
      }
    }
  }, [selectedIndex]);

  const getCategoryIcon = (category: string) => {
    switch (category.toLowerCase()) {
      case 'crypto':
        return <Hash className="w-4 h-4" />;
      case 'forex':
        return <DollarSign className="w-4 h-4" />;
      case 'commodity':
        return <TrendingUp className="w-4 h-4" />;
      default:
        return <Layers className="w-4 h-4" />;
    }
  };

  return (
    <div ref={containerRef} className={clsx("relative", className)}>
      <div className="relative">
        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-text-tertiary" size={16} />
        
        <input
          ref={inputRef}
          type="text"
          value={query || currentSymbol || ''}
          onChange={(e) => {
            setQuery(e.target.value);
            setIsOpen(true);
          }}
          onFocus={() => {
            if (query) setIsOpen(true);
          }}
          onKeyDown={handleKeyDown}
          className={clsx(
            "w-full pl-10 pr-10 py-2.5 bg-bg-secondary border border-border",
            "text-text-primary placeholder-text-tertiary",
            "focus:outline-none focus:border-accent-primary focus:ring-1 focus:ring-accent-primary",
            "transition-all duration-200"
          )}
          placeholder={placeholder}
        />
        
        {query ? (
          <button
            onClick={() => {
              setQuery('');
              setResults([]);
              inputRef.current?.focus();
            }}
            className="absolute right-3 top-1/2 transform -translate-y-1/2 text-text-tertiary hover:text-text-primary transition-colors"
          >
            <X size={16} />
          </button>
        ) : (
          <ChevronDown className="absolute right-3 top-1/2 transform -translate-y-1/2 text-text-tertiary" size={16} />
        )}
      </div>

      {/* Search Results Dropdown */}
      {isOpen && (query || results.length > 0) && (
        <div 
          ref={resultsRef}
          className={clsx(
            "absolute z-50 w-full mt-1 bg-bg-primary border border-border shadow-2xl",
            "max-h-96 overflow-y-auto",
            "animate-in fade-in slide-in-from-top-1 duration-200"
          )}
        >
          {isLoading && (
            <div className="p-4 text-center text-text-tertiary">
              <div className="inline-block animate-spin rounded-full h-5 w-5 border-b-2 border-accent-primary"></div>
              <span className="ml-2">Searching...</span>
            </div>
          )}

          {error && (
            <div className="p-4 text-center text-accent-red">
              {error}
            </div>
          )}

          {!isLoading && !error && results.length === 0 && query && (
            <div className="p-4 text-center text-text-tertiary">
              <div>No results found for "{query}"</div>
              {query.includes(' ') || query.includes('/') ? (
                <div className="text-xs mt-1 text-text-quaternary">
                  Searching for symbols containing all terms
                </div>
              ) : null}
            </div>
          )}

          {!isLoading && !error && results.length > 0 && (
            <div className="py-2">
              {results.map((result, index) => {
                const relevance = getRelevanceIndicator(result.relevance_score);
                const isSelected = index === selectedIndex;
                
                return (
                  <div
                    key={`${result.normalized_id}-${index}`}
                    data-result-item
                    onMouseEnter={() => setSelectedIndex(index)}
                    className={clsx(
                      "px-4 py-3 transition-all duration-150",
                      "border-b border-border/50 last:border-b-0",
                      isSelected ? "bg-bg-secondary" : ""
                    )}
                  >
                    {/* Main Row */}
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-3">
                        {/* Category Icon */}
                        <div className="text-text-tertiary">
                          {getCategoryIcon(result.category)}
                        </div>
                        
                        {/* Symbol Info */}
                        <div>
                          <div className="flex items-center gap-2">
                            <span className="text-text-primary font-semibold text-lg">
                              {result.normalized_id}
                            </span>
                            <span className="text-text-secondary text-sm">
                              {result.display_name}
                            </span>
                          </div>
                          <div className="text-text-tertiary text-xs mt-0.5">
                            {result.description}
                          </div>
                        </div>
                      </div>

                      {/* Relevance Score */}
                      <div className="flex flex-col items-end gap-1">
                        <span 
                          className="text-xs font-medium px-2 py-0.5 rounded"
                          style={{ 
                            backgroundColor: `${relevance.color}20`,
                            color: relevance.color 
                          }}
                        >
                          {relevance.label}
                        </span>
                        <div className="w-20 h-1 bg-bg-tertiary rounded-full overflow-hidden">
                          <div 
                            className="h-full transition-all duration-300"
                            style={{ 
                              width: `${relevance.percentage}%`,
                              backgroundColor: relevance.color 
                            }}
                          />
                        </div>
                      </div>
                    </div>

                    {/* Exchange Selection Grid */}
                    <div className="space-y-1.5">
                      <div className="text-xs text-text-tertiary mb-1">Available on:</div>
                      <div className="grid grid-cols-2 gap-2">
                        {result.exchanges.map((exchange, idx) => (
                          <button
                            key={`${exchange.exchange}-${idx}`}
                            onClick={() => handleSelectSymbol(result, exchange.symbol)}
                            className="flex items-center justify-between px-3 py-2 rounded text-sm transition-all duration-150 hover:scale-[1.02] active:scale-[0.98]"
                            style={{
                              backgroundColor: `${getExchangeColor(exchange.exchange)}10`,
                              border: `1px solid ${getExchangeColor(exchange.exchange)}30`,
                            }}
                            onMouseEnter={(e) => {
                              e.currentTarget.style.backgroundColor = `${getExchangeColor(exchange.exchange)}20`;
                              e.currentTarget.style.borderColor = `${getExchangeColor(exchange.exchange)}50`;
                            }}
                            onMouseLeave={(e) => {
                              e.currentTarget.style.backgroundColor = `${getExchangeColor(exchange.exchange)}10`;
                              e.currentTarget.style.borderColor = `${getExchangeColor(exchange.exchange)}30`;
                            }}
                          >
                            <span className="font-medium text-text-primary">
                              {formatExchangeName(exchange.exchange)}
                            </span>
                            <span className="text-text-tertiary text-xs font-mono">
                              {exchange.symbol}
                            </span>
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* Quick Actions Footer */}
          {!isLoading && results.length > 0 && (
            <div className="p-2 border-t border-border bg-bg-secondary/30">
              <div className="flex items-center justify-between text-xs text-text-tertiary">
                <span>
                  {results.length} result{results.length !== 1 ? 's' : ''} found
                  {(query.includes(' ') || query.includes('/')) && (
                    <span className="ml-1 text-accent-primary">(AND search)</span>
                  )}
                </span>
                <div className="flex items-center gap-3">
                  <span>↑↓ Navigate</span>
                  <span>↵ Select</span>
                  <span>ESC Close</span>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}