// Symbol API Service for interacting with the symbol search endpoint

export interface ExchangeSymbol {
  exchange: string;
  symbol: string;
}

export interface SearchResult {
  normalized_id: string;
  display_name: string;
  description: string;
  base: string;
  quote: string;
  category: string;
  exchanges: ExchangeSymbol[];
  relevance_score: number;
}

export interface SymbolSearchResponse {
  results: SearchResult[];
}

// Get API base URL from environment or use default
const API_BASE_URL = 'https://api.rednax.io';

// Cache for search results
const searchCache = new Map<string, { data: SearchResult[]; timestamp: number }>();
const CACHE_TTL = 60000; // 1 minute cache

/**
 * Search for symbols using the new symbol-search endpoint
 * @param query - The search query string
 * @returns Array of search results sorted by relevance
 */
export async function searchSymbols(query: string): Promise<SearchResult[]> {
  if (!query || query.trim().length === 0) {
    return [];
  }

  const normalizedQuery = query.trim().toLowerCase();
  
  // Check cache first
  const cached = searchCache.get(normalizedQuery);
  if (cached && Date.now() - cached.timestamp < CACHE_TTL) {
    return cached.data;
  }

  try {
    let results: SearchResult[] = [];
    
    // Check if user entered multiple terms separated by space or slash
    const terms = normalizedQuery.split(/[\s\/]+/).filter(term => term.length > 0);
    
    if (terms.length === 2) {
      // For two terms, try both orders since API only matches exact order
      // e.g., "usd btc" should also find "BTC/USD"
      const searches = [
        `${terms[0]}/${terms[1]}`, // Original order: btc/usd
        `${terms[1]}/${terms[0]}`  // Reversed order: usd/btc
      ];
      
      console.log(`Searching for pairs: ${searches.join(' and ')}`);
      
      // Try both orders in parallel
      const searchPromises = searches.map(searchQuery => 
        fetch(`${API_BASE_URL}/api/symbol-search?q=${encodeURIComponent(searchQuery)}`, {
          method: 'GET',
          headers: { 'Accept': 'application/json' },
        }).then(res => res.ok ? res.json() : { results: [] })
          .catch(() => ({ results: [] }))
      );
      
      const responses = await Promise.all(searchPromises);
      
      // Combine and deduplicate results
      const allResults = responses.flatMap(r => r.results || []);
      const uniqueResults = new Map<string, SearchResult>();
      
      allResults.forEach(result => {
        if (!uniqueResults.has(result.normalized_id)) {
          uniqueResults.set(result.normalized_id, result);
        }
      });
      
      results = Array.from(uniqueResults.values());
      
      // Sort by relevance score
      results.sort((a, b) => b.relevance_score - a.relevance_score);
    } else {
      // For single term or more than 2 terms, use original query
      console.log(`Searching for: ${normalizedQuery}`);
      const response = await fetch(
        `${API_BASE_URL}/api/symbol-search?q=${encodeURIComponent(normalizedQuery)}`,
        {
          method: 'GET',
          headers: { 'Accept': 'application/json' },
        }
      );

      if (!response.ok) {
        throw new Error(`Search failed: ${response.statusText}`);
      }

      const data: SymbolSearchResponse = await response.json();
      results = data.results;
    }

    // Cache the results
    searchCache.set(normalizedQuery, {
      data: results,
      timestamp: Date.now(),
    });

    return results;
  } catch (error) {
    console.error('Symbol search error:', error);
    return [];
  }
}

/**
 * Get all available symbols (no search query)
 * @returns Array of all symbols from all exchanges
 */
export async function getAllSymbols(): Promise<string[]> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/symbols`, {
      method: 'GET',
      headers: {
        'Accept': 'application/json',
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch symbols: ${response.statusText}`);
    }

    const data = await response.json();
    return data.symbols || [];
  } catch (error) {
    console.error('Failed to fetch all symbols:', error);
    return [];
  }
}

/**
 * Clear the search cache
 */
export function clearSearchCache(): void {
  searchCache.clear();
}

/**
 * Format exchange name for display
 */
export function formatExchangeName(exchange: string): string {
  const exchangeMap: Record<string, string> = {
    coinbase: 'Coinbase',
    binance: 'Binance',
    bitfinex: 'Bitfinex',
    kraken: 'Kraken',
    okx: 'OKX',
  };
  return exchangeMap[exchange.toLowerCase()] || exchange;
}

/**
 * Get exchange color for visual indicators
 */
export function getExchangeColor(exchange: string): string {
  const colorMap: Record<string, string> = {
    coinbase: '#0052FF',  // Coinbase blue
    binance: '#F3BA2F',   // Binance yellow
    bitfinex: '#5CDB95',  // Bitfinex green
    kraken: '#5741D9',    // Kraken purple
    okx: '#000000',       // OKX black
  };
  return colorMap[exchange.toLowerCase()] || '#6B7280';
}

/**
 * Group search results by category
 */
export function groupResultsByCategory(results: SearchResult[]): Record<string, SearchResult[]> {
  const grouped: Record<string, SearchResult[]> = {};

  results.forEach(result => {
    const category = result.category || 'Other';
    if (!grouped[category]) {
      grouped[category] = [];
    }
    grouped[category].push(result);
  });

  return grouped;
}

/**
 * Get a relevance indicator based on score
 */
export function getRelevanceIndicator(score: number): {
  label: string;
  color: string;
  percentage: number;
} {
  const maxScore = 150; // Maximum possible score
  const percentage = Math.min(100, (score / maxScore) * 100);

  if (score >= 120) {
    return { label: 'Exact Match', color: '#10B981', percentage }; // Green
  } else if (score >= 80) {
    return { label: 'High Match', color: '#3B82F6', percentage }; // Blue
  } else if (score >= 50) {
    return { label: 'Good Match', color: '#F59E0B', percentage }; // Yellow
  } else {
    return { label: 'Partial Match', color: '#6B7280', percentage }; // Gray
  }
}

/**
 * Parse exchange and symbol from a combined string
 * Format: "exchange:SYMBOL" or just "SYMBOL"
 */
export function parseSymbol(symbol: string): { exchange: string; baseSymbol: string } {
  if (symbol.includes(':')) {
    const [exchange, baseSymbol] = symbol.split(':');
    return { exchange, baseSymbol };
  }
  // Default to coinbase for backward compatibility
  return { exchange: 'coinbase', baseSymbol: symbol };
}

/**
 * Combine exchange and symbol into a single string
 */
export function combineSymbol(exchange: string, baseSymbol: string): string {
  return `${exchange}:${baseSymbol}`;
}