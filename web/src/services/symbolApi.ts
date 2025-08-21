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

  // Normalize the query for the API
  // Convert spaces to slashes for pair searching (e.g., "btc usd" -> "btc/usd")
  let searchQuery = query.trim();
  
  // Check if user entered multiple terms separated by space
  if (searchQuery.includes(' ') && !searchQuery.includes('/')) {
    // Convert "btc usd" to "btc/usd" for the API
    const terms = searchQuery.split(/\s+/).filter(term => term.length > 0);
    if (terms.length === 2) {
      // For exactly 2 terms, use slash notation which the API supports
      searchQuery = `${terms[0]}/${terms[1]}`;
    }
  }

  // Check cache first
  const cacheKey = searchQuery.toLowerCase();
  const cached = searchCache.get(cacheKey);
  if (cached && Date.now() - cached.timestamp < CACHE_TTL) {
    return cached.data;
  }

  try {
    console.log(`Searching for: ${searchQuery}`);
    const response = await fetch(
      `${API_BASE_URL}/api/symbol-search?q=${encodeURIComponent(searchQuery)}`,
      {
        method: 'GET',
        headers: {
          'Accept': 'application/json',
        },
      }
    );

    if (!response.ok) {
      throw new Error(`Search failed: ${response.statusText}`);
    }

    const data: SymbolSearchResponse = await response.json();

    // Cache the results
    searchCache.set(cacheKey, {
      data: data.results,
      timestamp: Date.now(),
    });

    return data.results;
  } catch (error) {
    console.error('Symbol search error:', error);
    // Return empty array on error to allow graceful degradation
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