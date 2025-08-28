import React, { useEffect, useRef, useState, useCallback } from 'react';
import { createPortal } from 'react-dom';

export interface TooltipData {
  x: number;
  y: number;
  time: string;
  timestamp?: number;
  price?: number;
  visible: boolean;
  // Optional additional data
  volume?: number;
  side?: 'buy' | 'sell';
  bestBid?: number;
  bestAsk?: number;
  exchange?: string;
  symbol?: string;
  change24h?: number;
}

export interface ChartTooltipProps {
  data: TooltipData | null;
  containerRef?: React.RefObject<HTMLElement>;
  usePortal?: boolean;
  showCrosshair?: boolean;
  followCursor?: boolean;
}

function formatPrice(price: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
  }).format(price);
}

function formatVolume(volume: number): string {
  if (volume >= 1e9) return `${(volume / 1e9).toFixed(2)}B`;
  if (volume >= 1e6) return `${(volume / 1e6).toFixed(2)}M`;  
  if (volume >= 1e3) return `${(volume / 1e3).toFixed(1)}K`;
  return volume.toFixed(2);
}

function formatTimestamp(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleString('en-US', {
    month: 'short',
    day: '2-digit', 
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

export const ChartTooltip: React.FC<ChartTooltipProps> = ({ 
  data, 
  containerRef, 
  usePortal = true,
  showCrosshair = true,
  followCursor = true
}) => {
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const [show, setShow] = useState(false);

  // Update tooltip position with smart boundary detection
  useEffect(() => {
    if (!data || !data.visible || !tooltipRef.current) {
      setShow(false);
      return;
    }

    const tooltip = tooltipRef.current;
    let finalX = data.x;
    let finalY = data.y;

    if (containerRef?.current) {
      const container = containerRef.current;
      const containerRect = container.getBoundingClientRect();
      
      // Get tooltip dimensions
      const tooltipRect = tooltip.getBoundingClientRect();
      const tooltipWidth = tooltipRect.width || 280;
      const tooltipHeight = tooltipRect.height || 150;
      
      // Smart positioning with quadrant detection
      const offsetX = 15;
      const offsetY = 15;
      const centerX = containerRect.width / 2;
      const centerY = containerRect.height / 2;
      
      // Position based on quadrant to minimize edge collisions
      if (data.x > centerX && data.y < centerY) {
        // Top-right quadrant: place tooltip left and below
        finalX = data.x - tooltipWidth - offsetX;
        finalY = data.y + offsetY;
      } else if (data.x <= centerX && data.y < centerY) {
        // Top-left quadrant: place tooltip right and below  
        finalX = data.x + offsetX;
        finalY = data.y + offsetY;
      } else if (data.x <= centerX && data.y >= centerY) {
        // Bottom-left quadrant: place tooltip right and above
        finalX = data.x + offsetX;
        finalY = data.y - tooltipHeight - offsetY;
      } else {
        // Bottom-right quadrant: place tooltip left and above
        finalX = data.x - tooltipWidth - offsetX;
        finalY = data.y - tooltipHeight - offsetY;
      }
      
      // Final boundary enforcement
      finalX = Math.max(10, Math.min(finalX, containerRect.width - tooltipWidth - 10));
      finalY = Math.max(10, Math.min(finalY, containerRect.height - tooltipHeight - 10));

      // Convert to absolute positioning if using portal
      if (usePortal) {
        finalX += containerRect.left;
        finalY += containerRect.top;
      }
    }

    setPosition({ x: finalX, y: finalY });
    setShow(true);
  }, [data, containerRef, usePortal]);

  if (!data || !data.visible || !show) {
    return null;
  }

  const changeColor = data.change24h !== undefined
    ? data.change24h >= 0 ? 'text-green-400' : 'text-red-400'
    : 'text-gray-400';

  const changeSign = data.change24h !== undefined && data.change24h >= 0 ? '+' : '';

  // Crosshair lines (if enabled)
  const crosshairLines = showCrosshair && containerRef?.current && (
    <>
      {/* Vertical line */}
      <div 
        className="absolute bg-gray-500 opacity-50 pointer-events-none z-[9998]"
        style={{
          left: `${data.x}px`,
          top: 0,
          width: '1px',
          height: '100%',
        }}
      />
      {/* Horizontal line */}
      <div 
        className="absolute bg-gray-500 opacity-50 pointer-events-none z-[9998]"
        style={{
          left: 0,
          top: `${data.y}px`,
          width: '100%',
          height: '1px',
        }}
      />
    </>
  );

  const tooltipContent = (
    <>
      {!usePortal && crosshairLines}
      <div
        ref={tooltipRef}
        className="bg-gray-900 border border-gray-600 rounded-lg shadow-2xl p-4 max-w-xs pointer-events-none z-[9999]"
        style={{
          position: usePortal ? 'fixed' : 'absolute',
          left: `${position.x}px`,
          top: `${position.y}px`,
          transform: show ? 'scale(1)' : 'scale(0.95)',
          opacity: show ? 1 : 0,
          transition: followCursor ? 'none' : 'all 0.15s ease-out',
        }}
        data-testid="chart-tooltip"
        role="tooltip"
        aria-live="polite"
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-3">
          {data.symbol && (
            <div className="text-white font-semibold text-sm">
              {data.symbol}
              {data.exchange && (
                <span className="text-gray-400 text-xs ml-2">
                  {data.exchange}
                </span>
              )}
            </div>
          )}
          <div className="text-xs text-gray-500">
            {data.timestamp ? formatTimestamp(data.timestamp) : data.time}
          </div>
        </div>

        {/* Price Data */}
        <div className="space-y-2">
          {data.price !== undefined && (
            <div className="flex items-center justify-between">
              <span className="text-gray-400 text-sm">Price</span>
              <span className="text-white font-mono font-medium">
                {formatPrice(data.price)}
              </span>
            </div>
          )}

          {data.change24h !== undefined && (
            <div className="flex items-center justify-between">
              <span className="text-gray-400 text-sm">24h Change</span>
              <span className={`font-mono font-medium ${changeColor}`}>
                {changeSign}{data.change24h.toFixed(2)}%
              </span>
            </div>
          )}

          {data.volume !== undefined && (
            <div className="flex items-center justify-between">
              <span className="text-gray-400 text-sm">Volume</span>
              <span className="text-white font-mono">
                {formatVolume(data.volume)}
              </span>
            </div>
          )}

          {data.side && (
            <div className="flex items-center justify-between">
              <span className="text-gray-400 text-sm">Side</span>
              <span className={`font-mono font-medium uppercase ${
                data.side === 'buy' ? 'text-green-400' : 'text-red-400'
              }`}>
                {data.side}
              </span>
            </div>
          )}

          {data.bestBid !== undefined && data.bestAsk !== undefined && (
            <>
              <div className="flex items-center justify-between">
                <span className="text-gray-400 text-sm">Best Bid</span>
                <span className="text-green-400 font-mono">
                  ${formatPrice(data.bestBid).replace('$', '')}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-gray-400 text-sm">Best Ask</span>
                <span className="text-red-400 font-mono">
                  ${formatPrice(data.bestAsk).replace('$', '')}
                </span>
              </div>
            </>
          )}
        </div>

        {/* Coordinate Info (for debugging) */}
        <div className="mt-3 pt-3 border-t border-gray-700">
          <div className="grid grid-cols-2 gap-2 text-xs text-gray-500">
            <div>X: {data.x.toFixed(0)}</div>
            <div>Y: {data.y.toFixed(0)}</div>
          </div>
        </div>
      </div>
    </>
  );

  // Render with portal or inline
  if (usePortal) {
    return createPortal(
      <>
        {showCrosshair && containerRef?.current && createPortal(crosshairLines, containerRef.current)}
        {tooltipContent}
      </>,
      document.body
    );
  }

  return tooltipContent;
};

// Modern hover-based tooltip provider
export interface TooltipProviderProps {
  children: React.ReactNode;
  onTooltipData?: (data: TooltipData | null) => void;
  disabled?: boolean;
  hoverDelay?: number;
  hideDelay?: number;
  enableKeyboardTooltip?: boolean;
}

export const TooltipProvider: React.FC<TooltipProviderProps> = ({
  children,
  onTooltipData,
  disabled = false,
  hoverDelay = 300,
  hideDelay = 100,
  enableKeyboardTooltip = true,
}) => {
  const [tooltipData, setTooltipData] = useState<TooltipData | null>(null);
  const [isVisible, setIsVisible] = useState(false);
  const [containerRect, setContainerRect] = useState<DOMRect | null>(null);
  
  const containerRef = useRef<HTMLDivElement>(null);
  const showTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const hideTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const lastMousePosRef = useRef<{ x: number; y: number } | null>(null);

  const showTooltip = useCallback((data: TooltipData) => {
    if (disabled) return;
    
    // Clear hide timeout if showing
    if (hideTimeoutRef.current) {
      clearTimeout(hideTimeoutRef.current);
      hideTimeoutRef.current = null;
    }
    
    setTooltipData(data);
    setIsVisible(true);
    onTooltipData?.(data);
  }, [disabled, onTooltipData]);

  const hideTooltip = useCallback((delay = hideDelay) => {
    // Clear show timeout if hiding
    if (showTimeoutRef.current) {
      clearTimeout(showTimeoutRef.current);
      showTimeoutRef.current = null;
    }
    
    if (hideTimeoutRef.current) {
      clearTimeout(hideTimeoutRef.current);
    }
    
    hideTimeoutRef.current = setTimeout(() => {
      setIsVisible(false);
      setTooltipData(null);
      onTooltipData?.(null);
    }, delay);
  }, [hideDelay, onTooltipData]);

  const handleMouseMove = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    if (disabled) return;
    
    const rect = event.currentTarget.getBoundingClientRect();
    setContainerRect(rect);
    
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    lastMousePosRef.current = { x, y };
    
    // Clear existing show timeout
    if (showTimeoutRef.current) {
      clearTimeout(showTimeoutRef.current);
    }
    
    // Show tooltip after delay (professional UX pattern)
    showTimeoutRef.current = setTimeout(() => {
      const currentPos = lastMousePosRef.current;
      if (currentPos && Math.abs(currentPos.x - x) < 5 && Math.abs(currentPos.y - y) < 5) {
        // Create sample tooltip data (in real implementation, this would come from WASM)
        const sampleData: TooltipData = {
          x,
          y,
          time: new Date().toLocaleTimeString(),
          timestamp: Math.floor(Date.now() / 1000),
          price: 45678.90 + Math.random() * 1000,
          volume: 1234567.89,
          exchange: 'Coinbase',
          symbol: 'BTC-USD',
          change24h: (Math.random() - 0.5) * 10,
          visible: true,
        };
        
        showTooltip(sampleData);
      }
    }, hoverDelay);
  }, [disabled, hoverDelay, showTooltip]);

  const handleMouseLeave = useCallback(() => {
    hideTooltip(hideDelay * 3); // Longer delay when leaving entirely
  }, [hideTooltip, hideDelay]);

  const handleKeyDown = useCallback((event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!enableKeyboardTooltip || disabled) return;
    
    // Show tooltip on Alt key (accessibility feature)
    if (event.altKey && !event.repeat) {
      const rect = containerRef.current?.getBoundingClientRect();
      if (rect) {
        const centerX = rect.width / 2;
        const centerY = rect.height / 2;
        
        const keyboardData: TooltipData = {
          x: centerX,
          y: centerY,
          time: new Date().toLocaleTimeString(),
          timestamp: Math.floor(Date.now() / 1000),
          price: 45678.90,
          volume: 1234567.89,
          exchange: 'Coinbase',
          symbol: 'BTC-USD',
          change24h: 2.34,
          visible: true,
        };
        
        showTooltip(keyboardData);
      }
    }
  }, [enableKeyboardTooltip, disabled, showTooltip]);

  const handleKeyUp = useCallback((event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!enableKeyboardTooltip || disabled) return;
    
    // Hide tooltip when Alt key released
    if (!event.altKey) {
      hideTooltip(0);
    }
  }, [enableKeyboardTooltip, disabled, hideTooltip]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (showTimeoutRef.current) clearTimeout(showTimeoutRef.current);
      if (hideTimeoutRef.current) clearTimeout(hideTimeoutRef.current);
    };
  }, []);

  return (
    <>
      <div
        ref={containerRef}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
        onKeyDown={handleKeyDown}
        onKeyUp={handleKeyUp}
        tabIndex={0}
        className="relative focus:outline-none"
        style={{ cursor: disabled ? 'default' : 'crosshair' }}
        aria-label="Chart area - hover to see data details, hold Alt key for keyboard tooltip"
      >
        {children}
      </div>
      
      <ChartTooltip
        data={tooltipData}
        containerRef={containerRef}
        usePortal={true}
        showCrosshair={true}
        followCursor={true}
      />
    </>
  );
};