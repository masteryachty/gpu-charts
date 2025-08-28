import React from 'react';

/**
 * Accessible indicator component that provides color, icon, and text alternatives
 * Ensures information is conveyed through multiple channels, not just color
 */

interface AccessibleIndicatorProps {
  /** Primary text/label */
  label: string;
  
  /** Color for visual users */
  color: string;
  
  /** Icon or emoji alternative */
  icon?: string;
  
  /** Additional text for context */
  text?: string;
  
  /** Screen reader description */
  ariaLabel?: string;
  
  /** Visual pattern for additional distinction */
  pattern?: 'solid' | 'diagonal' | 'dots' | 'waves' | 'hexagon';
  
  /** Size variant */
  size?: 'sm' | 'md' | 'lg';
  
  /** Shape variant */
  shape?: 'circle' | 'square' | 'badge' | 'dot';
  
  /** Additional CSS classes */
  className?: string;
  
  /** Click handler */
  onClick?: () => void;
  
  /** Whether indicator is interactive */
  interactive?: boolean;
}

const patternStyles = {
  solid: {},
  diagonal: {
    backgroundImage: 'repeating-linear-gradient(45deg, transparent, transparent 2px, rgba(255,255,255,0.1) 2px, rgba(255,255,255,0.1) 4px)'
  },
  dots: {
    backgroundImage: 'radial-gradient(circle at 50% 50%, rgba(255,255,255,0.2) 1px, transparent 1px)',
    backgroundSize: '6px 6px'
  },
  waves: {
    backgroundImage: 'repeating-linear-gradient(90deg, transparent, transparent 3px, rgba(255,255,255,0.1) 3px, rgba(255,255,255,0.1) 6px)'
  },
  hexagon: {
    clipPath: 'polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%)'
  }
};

const sizeStyles = {
  sm: 'text-xs px-1.5 py-0.5',
  md: 'text-sm px-2 py-1',
  lg: 'text-base px-3 py-1.5'
};

const shapeStyles = {
  circle: 'rounded-full',
  square: 'rounded',
  badge: 'rounded-full px-2',
  dot: 'rounded-full w-3 h-3 min-w-3'
};

export function AccessibleIndicator({
  label,
  color,
  icon,
  text,
  ariaLabel,
  pattern = 'solid',
  size = 'md',
  shape = 'badge',
  className = '',
  onClick,
  interactive = false
}: AccessibleIndicatorProps) {
  const Component = onClick || interactive ? 'button' : 'span';
  
  const baseClasses = [
    'inline-flex items-center gap-1 transition-all duration-200',
    sizeStyles[size],
    shapeStyles[shape],
    interactive && 'hover:opacity-80 focus:outline-none focus:ring-2 focus:ring-blue-500',
    className
  ].filter(Boolean).join(' ');

  const style = {
    backgroundColor: color,
    color: getContrastColor(color),
    border: `1px solid ${adjustColor(color, -20)}`,
    ...patternStyles[pattern]
  };

  return (
    <Component
      className={baseClasses}
      style={style}
      onClick={onClick}
      aria-label={ariaLabel || `${label}${text ? `: ${text}` : ''}`}
      role={onClick ? 'button' : undefined}
      tabIndex={interactive ? 0 : undefined}
    >
      {icon && (
        <span className="flex-shrink-0" aria-hidden="true">
          {icon}
        </span>
      )}
      {shape !== 'dot' && (
        <>
          <span className="font-medium">{label}</span>
          {text && <span className="opacity-90">{text}</span>}
        </>
      )}
      {shape === 'dot' && (
        <span className="sr-only">{label}{text && `: ${text}`}</span>
      )}
    </Component>
  );
}

/**
 * Exchange indicator with icon and text alternatives
 */
interface ExchangeIndicatorProps {
  exchange: string;
  symbol?: string;
  active?: boolean;
  onClick?: () => void;
  size?: 'sm' | 'md' | 'lg';
}

export function ExchangeIndicator({ 
  exchange, 
  symbol, 
  active = false, 
  onClick,
  size = 'md' 
}: ExchangeIndicatorProps) {
  const exchangeInfo = getExchangeInfo(exchange);
  
  return (
    <AccessibleIndicator
      label={exchangeInfo.name}
      color={active ? exchangeInfo.color : '#6B7280'}
      icon={exchangeInfo.icon}
      text={symbol}
      ariaLabel={`${exchangeInfo.name}${symbol ? ` ${symbol}` : ''}${active ? ' (active)' : ''}`}
      pattern={active ? exchangeInfo.pattern : 'solid'}
      size={size}
      onClick={onClick}
      interactive={!!onClick}
      className={active ? 'ring-2 ring-blue-500' : ''}
    />
  );
}

/**
 * Status indicator with accessible alternatives
 */
interface StatusIndicatorProps {
  status: 'active' | 'inactive' | 'loading' | 'error' | 'success' | 'warning';
  text?: string;
  size?: 'sm' | 'md' | 'lg';
  shape?: 'circle' | 'square' | 'badge' | 'dot';
}

export function StatusIndicator({ 
  status, 
  text, 
  size = 'sm', 
  shape = 'dot' 
}: StatusIndicatorProps) {
  const statusInfo = getStatusIndicator(status);
  
  return (
    <AccessibleIndicator
      label={statusInfo.text}
      color={statusInfo.color}
      icon={statusInfo.icon}
      text={text}
      ariaLabel={statusInfo.ariaLabel}
      size={size}
      shape={shape}
    />
  );
}

/**
 * Price change indicator with directional alternatives
 */
interface PriceChangeIndicatorProps {
  change: number;
  percentage: number;
  size?: 'sm' | 'md' | 'lg';
}

export function PriceChangeIndicator({ 
  change, 
  percentage, 
  size = 'md' 
}: PriceChangeIndicatorProps) {
  const priceInfo = getPriceChangeIndicator(change, percentage);
  
  return (
    <AccessibleIndicator
      label={priceInfo.direction}
      color={priceInfo.color}
      icon={priceInfo.icon}
      text={priceInfo.text}
      ariaLabel={priceInfo.ariaLabel}
      size={size}
      pattern={priceInfo.direction === 'up' ? 'diagonal' : priceInfo.direction === 'down' ? 'waves' : 'solid'}
    />
  );
}

// Helper functions
function getContrastColor(hexColor: string): string {
  // Remove # if present
  const color = hexColor.replace('#', '');
  
  // Convert to RGB
  const r = parseInt(color.substr(0, 2), 16);
  const g = parseInt(color.substr(2, 2), 16);
  const b = parseInt(color.substr(4, 2), 16);
  
  // Calculate luminance
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  
  return luminance > 0.5 ? '#000000' : '#FFFFFF';
}

function adjustColor(hexColor: string, percent: number): string {
  const color = hexColor.replace('#', '');
  const num = parseInt(color, 16);
  
  const amt = Math.round(2.55 * percent);
  const R = (num >> 16) + amt;
  const G = (num >> 8 & 0x00FF) + amt;
  const B = (num & 0x0000FF) + amt;
  
  return `#${(0x1000000 + (R < 255 ? R < 1 ? 0 : R : 255) * 0x10000 +
    (G < 255 ? G < 1 ? 0 : G : 255) * 0x100 +
    (B < 255 ? B < 1 ? 0 : B : 255)).toString(16).slice(1)}`;
}

// Import helper functions from utility
function getExchangeInfo(exchange: string) {
  // This would normally import from utils/exchangeIcons
  const exchangeMap = {
    coinbase: { name: 'Coinbase', color: '#0066FF', icon: '🏪', pattern: 'solid' },
    binance: { name: 'Binance', color: '#FFD700', icon: '⚡', pattern: 'diagonal' },
    bitfinex: { name: 'Bitfinex', color: '#00FF88', icon: '💹', pattern: 'dots' },
    kraken: { name: 'Kraken', color: '#9945FF', icon: '🐙', pattern: 'waves' },
    okx: { name: 'OKX', color: '#FF00FF', icon: '🔷', pattern: 'hexagon' },
  } as any;
  
  return exchangeMap[exchange.toLowerCase()] || {
    name: exchange,
    color: '#FF6B6B',
    icon: '📊',
    pattern: 'solid'
  };
}

function getStatusIndicator(status: string) {
  const statusMap = {
    active: { color: '#10B981', icon: '●', text: 'Active', ariaLabel: 'Status: Active' },
    inactive: { color: '#6B7280', icon: '○', text: 'Inactive', ariaLabel: 'Status: Inactive' },
    loading: { color: '#3B82F6', icon: '⏳', text: 'Loading', ariaLabel: 'Status: Loading' },
    error: { color: '#EF4444', icon: '⚠️', text: 'Error', ariaLabel: 'Status: Error' },
    success: { color: '#10B981', icon: '✓', text: 'Success', ariaLabel: 'Status: Success' },
    warning: { color: '#F59E0B', icon: '⚠️', text: 'Warning', ariaLabel: 'Status: Warning' }
  } as any;
  
  return statusMap[status] || statusMap.inactive;
}

function getPriceChangeIndicator(change: number, percentage: number) {
  if (change > 0) {
    return {
      color: '#10B981',
      icon: '📈',
      text: `+${change.toFixed(2)} (+${percentage.toFixed(2)}%)`,
      ariaLabel: `Price increased by ${change.toFixed(2)}, up ${percentage.toFixed(2)} percent`,
      direction: 'up' as const
    };
  } else if (change < 0) {
    return {
      color: '#EF4444',
      icon: '📉',
      text: `${change.toFixed(2)} (${percentage.toFixed(2)}%)`,
      ariaLabel: `Price decreased by ${Math.abs(change).toFixed(2)}, down ${Math.abs(percentage).toFixed(2)} percent`,
      direction: 'down' as const
    };
  } else {
    return {
      color: '#6B7280',
      icon: '➖',
      text: '0.00 (0.00%)',
      ariaLabel: 'Price unchanged',
      direction: 'neutral' as const
    };
  }
}

export default AccessibleIndicator;