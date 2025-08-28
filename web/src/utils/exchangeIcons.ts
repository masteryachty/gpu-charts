/**
 * Exchange-specific icons and visual indicators
 * Provides text and icon alternatives to color-only indicators
 */

export interface ExchangeInfo {
  name: string;
  color: string;
  icon: string;
  symbol: string;
  pattern?: string;
}

/**
 * Get comprehensive exchange information including icon alternatives
 */
export function getExchangeInfo(exchange: string): ExchangeInfo {
  const exchangeMap: Record<string, ExchangeInfo> = {
    coinbase: {
      name: 'Coinbase',
      color: '#0066FF',
      icon: '🏪', // Store/marketplace icon
      symbol: 'CB',
      pattern: 'solid'
    },
    binance: {
      name: 'Binance',
      color: '#FFD700',
      icon: '⚡', // Lightning bolt for speed
      symbol: 'BN',
      pattern: 'diagonal'
    },
    bitfinex: {
      name: 'Bitfinex',
      color: '#00FF88',
      icon: '💹', // Chart trending up
      symbol: 'BF',
      pattern: 'dots'
    },
    kraken: {
      name: 'Kraken',
      color: '#9945FF',
      icon: '🐙', // Octopus/sea creature
      symbol: 'KR',
      pattern: 'waves'
    },
    okx: {
      name: 'OKX',
      color: '#FF00FF',
      icon: '🔷', // Diamond shape
      symbol: 'OK',
      pattern: 'hexagon'
    },
  };
  
  return exchangeMap[exchange.toLowerCase()] || {
    name: exchange,
    color: '#FF6B6B',
    icon: '📊',
    symbol: 'EX',
    pattern: 'solid'
  };
}

/**
 * Get status indicators with text alternatives
 */
export function getStatusIndicator(status: 'active' | 'inactive' | 'loading' | 'error' | 'success' | 'warning'): {
  color: string;
  icon: string;
  text: string;
  ariaLabel: string;
} {
  const statusMap = {
    active: {
      color: '#10B981', // Green
      icon: '●',
      text: 'Active',
      ariaLabel: 'Status: Active'
    },
    inactive: {
      color: '#6B7280', // Gray
      icon: '○',
      text: 'Inactive',
      ariaLabel: 'Status: Inactive'
    },
    loading: {
      color: '#3B82F6', // Blue
      icon: '⏳',
      text: 'Loading',
      ariaLabel: 'Status: Loading'
    },
    error: {
      color: '#EF4444', // Red
      icon: '⚠️',
      text: 'Error',
      ariaLabel: 'Status: Error'
    },
    success: {
      color: '#10B981', // Green
      icon: '✓',
      text: 'Success',
      ariaLabel: 'Status: Success'
    },
    warning: {
      color: '#F59E0B', // Yellow
      icon: '⚠️',
      text: 'Warning',
      ariaLabel: 'Status: Warning'
    }
  };
  
  return statusMap[status];
}

/**
 * Get price change indicators with text alternatives
 */
export function getPriceChangeIndicator(change: number, percentage: number): {
  color: string;
  icon: string;
  text: string;
  ariaLabel: string;
  direction: 'up' | 'down' | 'neutral';
} {
  if (change > 0) {
    return {
      color: '#10B981', // Green
      icon: '📈',
      text: `+${change.toFixed(2)} (+${percentage.toFixed(2)}%)`,
      ariaLabel: `Price increased by ${change.toFixed(2)}, up ${percentage.toFixed(2)} percent`,
      direction: 'up'
    };
  } else if (change < 0) {
    return {
      color: '#EF4444', // Red
      icon: '📉',
      text: `${change.toFixed(2)} (${percentage.toFixed(2)}%)`,
      ariaLabel: `Price decreased by ${Math.abs(change).toFixed(2)}, down ${Math.abs(percentage).toFixed(2)} percent`,
      direction: 'down'
    };
  } else {
    return {
      color: '#6B7280', // Gray
      icon: '➖',
      text: '0.00 (0.00%)',
      ariaLabel: 'Price unchanged',
      direction: 'neutral'
    };
  }
}

/**
 * Get volume indicators with text alternatives
 */
export function getVolumeIndicator(volume: number, trend: 'high' | 'medium' | 'low'): {
  color: string;
  icon: string;
  text: string;
  ariaLabel: string;
} {
  const trendMap = {
    high: {
      color: '#EF4444', // Red for high activity
      icon: '🔥',
      text: 'High Volume',
      ariaLabel: `High trading volume: ${volume.toLocaleString()}`
    },
    medium: {
      color: '#F59E0B', // Orange for medium activity
      icon: '📊',
      text: 'Medium Volume',
      ariaLabel: `Medium trading volume: ${volume.toLocaleString()}`
    },
    low: {
      color: '#6B7280', // Gray for low activity
      icon: '📉',
      text: 'Low Volume',
      ariaLabel: `Low trading volume: ${volume.toLocaleString()}`
    }
  };
  
  return trendMap[trend];
}

/**
 * Get connection status indicators
 */
export function getConnectionStatus(status: 'connected' | 'connecting' | 'disconnected' | 'error'): {
  color: string;
  icon: string;
  text: string;
  ariaLabel: string;
} {
  const statusMap = {
    connected: {
      color: '#10B981', // Green
      icon: '🟢',
      text: 'Connected',
      ariaLabel: 'Connection status: Connected'
    },
    connecting: {
      color: '#F59E0B', // Yellow
      icon: '🟡',
      text: 'Connecting',
      ariaLabel: 'Connection status: Connecting'
    },
    disconnected: {
      color: '#6B7280', // Gray
      icon: '⚪',
      text: 'Disconnected',
      ariaLabel: 'Connection status: Disconnected'
    },
    error: {
      color: '#EF4444', // Red
      icon: '🔴',
      text: 'Connection Error',
      ariaLabel: 'Connection status: Error'
    }
  };
  
  return statusMap[status];
}

/**
 * Generate CSS pattern for better visual distinction
 */
export function generatePattern(patternType: string): string {
  const patterns = {
    solid: 'none',
    diagonal: 'repeating-linear-gradient(45deg, transparent, transparent 2px, rgba(255,255,255,0.1) 2px, rgba(255,255,255,0.1) 4px)',
    dots: 'radial-gradient(circle at 50% 50%, rgba(255,255,255,0.2) 1px, transparent 1px)',
    waves: 'repeating-linear-gradient(90deg, transparent, transparent 3px, rgba(255,255,255,0.1) 3px, rgba(255,255,255,0.1) 6px)',
    hexagon: 'conic-gradient(from 0deg, transparent 60deg, rgba(255,255,255,0.1) 60deg, rgba(255,255,255,0.1) 120deg, transparent 120deg)'
  };
  
  return patterns[patternType as keyof typeof patterns] || patterns.solid;
}

export default {
  getExchangeInfo,
  getStatusIndicator,
  getPriceChangeIndicator,
  getVolumeIndicator,
  getConnectionStatus,
  generatePattern
};