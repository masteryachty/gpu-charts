import React, { useEffect, useRef } from 'react';

export interface TooltipData {
  x: number;
  y: number;
  time: string;
  visible: boolean;
  // Optional additional data
  volume?: number;
  side?: 'buy' | 'sell';
  bestBid?: number;
  bestAsk?: number;
}

export interface ChartTooltipProps {
  data: TooltipData | null;
  containerRef?: React.RefObject<HTMLElement>;
}

export const ChartTooltip: React.FC<ChartTooltipProps> = ({ data, containerRef }) => {
  const tooltipRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!data || !data.visible || !tooltipRef.current) return;

    const tooltip = tooltipRef.current;
    
    // Position tooltip near the cursor with offset
    const offsetX = 15;
    const offsetY = 15;
    
    // Calculate initial position
    let left = data.x + offsetX;
    let top = data.y + offsetY;
    
    // If we have a container ref, adjust position to stay within bounds
    if (containerRef?.current) {
      const container = containerRef.current;
      const containerRect = container.getBoundingClientRect();
      
      // Get tooltip dimensions
      const tooltipRect = tooltip.getBoundingClientRect();
      const tooltipWidth = tooltipRect.width || 160; // Use min-width as fallback
      const tooltipHeight = tooltipRect.height || 100; // Estimated height
      
      // Adjust if tooltip would go off the right edge
      if (left + tooltipWidth > containerRect.width) {
        left = data.x - tooltipWidth - offsetX;
      }
      
      // Adjust if tooltip would go off the bottom edge
      if (top + tooltipHeight > containerRect.height) {
        top = data.y - tooltipHeight - offsetY;
      }
      
      // Ensure tooltip stays within bounds
      left = Math.max(0, Math.min(left, containerRect.width - tooltipWidth));
      top = Math.max(0, Math.min(top, containerRect.height - tooltipHeight));
    }
    
    tooltip.style.left = `${left}px`;
    tooltip.style.top = `${top}px`;
  }, [data, containerRef]);

  if (!data || !data.visible) {
    return null;
  }

  const formatPrice = (price: number): string => {
    return price.toLocaleString('en-US', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    });
  };

  const formatVolume = (volume: number): string => {
    if (volume >= 1000000) {
      return `${(volume / 1000000).toFixed(2)}M`;
    } else if (volume >= 1000) {
      return `${(volume / 1000).toFixed(2)}K`;
    }
    return volume.toFixed(4);
  };

  return (
    <div
      ref={tooltipRef}
      className="chart-tooltip"
      style={{
        position: 'absolute',
        pointerEvents: 'none',
        zIndex: 1000,
        backgroundColor: 'rgba(0, 0, 0, 0.9)',
        border: '1px solid rgba(255, 255, 255, 0.2)',
        borderRadius: '4px',
        padding: '8px 12px',
        fontSize: '12px',
        color: '#fff',
        boxShadow: '0 2px 8px rgba(0, 0, 0, 0.5)',
        minWidth: '160px',
        transition: 'none', // No transition for smooth following
      }}
      data-testid="chart-tooltip"
    >
      <div style={{ marginBottom: '4px' }}>
        <span style={{ color: '#9CA3AF' }}>Time: </span>
        <span style={{ color: '#fff' }}>{data.time}</span>
      </div>
      
      {data.volume !== undefined && (
        <div style={{ marginBottom: '4px' }}>
          <span style={{ color: '#9CA3AF' }}>Volume: </span>
          <span style={{ color: '#fff' }}>{formatVolume(data.volume)}</span>
        </div>
      )}
      
      {data.side && (
        <div style={{ marginBottom: '4px' }}>
          <span style={{ color: '#9CA3AF' }}>Side: </span>
          <span style={{ 
            color: data.side === 'buy' ? '#10B981' : '#EF4444',
            textTransform: 'uppercase'
          }}>
            {data.side.toUpperCase()}
          </span>
        </div>
      )}
      
      {data.bestBid !== undefined && data.bestAsk !== undefined && (
        <>
          <div style={{ marginBottom: '4px' }}>
            <span style={{ color: '#9CA3AF' }}>Bid: </span>
            <span style={{ color: '#10B981' }}>${formatPrice(data.bestBid)}</span>
          </div>
          <div>
            <span style={{ color: '#9CA3AF' }}>Ask: </span>
            <span style={{ color: '#EF4444' }}>${formatPrice(data.bestAsk)}</span>
          </div>
        </>
      )}
    </div>
  );
};