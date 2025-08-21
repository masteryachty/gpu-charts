import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ChartTooltip, TooltipData } from './ChartTooltip';
import React from 'react';

describe('ChartTooltip', () => {
  const mockContainerRef = React.createRef<HTMLDivElement>();
  
  beforeEach(() => {
    // Create a mock container element
    const container = document.createElement('div');
    container.style.width = '800px';
    container.style.height = '600px';
    container.style.position = 'relative';
    document.body.appendChild(container);
    Object.defineProperty(mockContainerRef, 'current', {
      writable: true,
      value: container
    });
  });

  describe('Visibility', () => {
    it('should not render when data is null', () => {
      const { container } = render(<ChartTooltip data={null} />);
      expect(container.querySelector('.chart-tooltip')).toBeNull();
    });

    it('should not render when visible is false', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15 10:30:00',
        price: 50000,
        visible: false
      };
      const { container } = render(<ChartTooltip data={data} />);
      expect(container.querySelector('.chart-tooltip')).toBeNull();
    });

    it('should render when visible is true', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15 10:30:00',
        price: 50000,
        visible: true
      };
      render(<ChartTooltip data={data} />);
      expect(screen.getByTestId('chart-tooltip')).toBeInTheDocument();
    });
  });

  describe('Content Display', () => {
    it('should display time and price', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15 10:30:00',
        price: 50000.50,
        visible: true
      };
      render(<ChartTooltip data={data} />);
      
      expect(screen.getByText('Time:')).toBeInTheDocument();
      expect(screen.getByText('2024-01-15 10:30:00')).toBeInTheDocument();
      expect(screen.getByText('Price:')).toBeInTheDocument();
      expect(screen.getByText('$50,000.50')).toBeInTheDocument();
    });

    it('should display volume when provided', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15 10:30:00',
        price: 50000,
        volume: 1234567.89,
        visible: true
      };
      render(<ChartTooltip data={data} />);
      
      expect(screen.getByText('Volume:')).toBeInTheDocument();
      expect(screen.getByText('1.23M')).toBeInTheDocument();
    });

    it('should display side when provided', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15 10:30:00',
        price: 50000,
        side: 'buy',
        visible: true
      };
      render(<ChartTooltip data={data} />);
      
      expect(screen.getByText('Side:')).toBeInTheDocument();
      expect(screen.getByText('BUY')).toBeInTheDocument();
    });

    it('should display bid and ask when provided', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15 10:30:00',
        price: 50000,
        bestBid: 49999.50,
        bestAsk: 50000.50,
        visible: true
      };
      render(<ChartTooltip data={data} />);
      
      expect(screen.getByText('Bid:')).toBeInTheDocument();
      expect(screen.getByText('$49,999.50')).toBeInTheDocument();
      expect(screen.getByText('Ask:')).toBeInTheDocument();
      expect(screen.getByText('$50,000.50')).toBeInTheDocument();
    });

    it('should display all fields when all data is provided', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15 10:30:00',
        price: 50000,
        volume: 500000,
        side: 'sell',
        bestBid: 49999,
        bestAsk: 50001,
        visible: true
      };
      render(<ChartTooltip data={data} />);
      
      expect(screen.getByText('Time:')).toBeInTheDocument();
      expect(screen.getByText('Price:')).toBeInTheDocument();
      expect(screen.getByText('Volume:')).toBeInTheDocument();
      expect(screen.getByText('Side:')).toBeInTheDocument();
      expect(screen.getByText('Bid:')).toBeInTheDocument();
      expect(screen.getByText('Ask:')).toBeInTheDocument();
    });
  });

  describe('Formatting', () => {
    it('should format large volumes correctly', () => {
      const testCases = [
        { volume: 1234567, expected: '1.23M' },
        { volume: 500000, expected: '500.00K' },
        { volume: 1500, expected: '1.50K' },
        { volume: 999, expected: '999.0000' },
        { volume: 0.1234, expected: '0.1234' }
      ];

      testCases.forEach(({ volume, expected }) => {
        const data: TooltipData = {
          x: 100,
          y: 100,
          time: '2024-01-15',
          price: 50000,
          volume,
          visible: true
        };
        const { rerender } = render(<ChartTooltip data={data} />);
        expect(screen.getByText(expected)).toBeInTheDocument();
        rerender(<ChartTooltip data={null} />);
      });
    });

    it('should format prices with proper decimal places', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15',
        price: 1234.5,
        visible: true
      };
      render(<ChartTooltip data={data} />);
      expect(screen.getByText('$1,234.50')).toBeInTheDocument();
    });

    it('should apply correct colors for buy/sell sides', () => {
      const buyData: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15',
        price: 50000,
        side: 'buy',
        visible: true
      };
      
      const { rerender } = render(<ChartTooltip data={buyData} />);
      const buyElement = screen.getByText('BUY');
      expect(buyElement).toHaveStyle({ color: '#10B981' }); // Green
      
      const sellData: TooltipData = { ...buyData, side: 'sell' };
      rerender(<ChartTooltip data={sellData} />);
      const sellElement = screen.getByText('SELL');
      expect(sellElement).toHaveStyle({ color: '#EF4444' }); // Red
    });
  });

  describe('Positioning', () => {
    it('should position tooltip at specified coordinates with offset', () => {
      const data: TooltipData = {
        x: 100,
        y: 200,
        time: '2024-01-15',
        price: 50000,
        visible: true
      };
      
      render(<ChartTooltip data={data} />);
      const tooltip = screen.getByTestId('chart-tooltip');
      
      // Tooltip should be positioned with a 15px offset when no container ref
      expect(tooltip).toHaveStyle({
        left: '115px',
        top: '215px'
      });
    });

    it('should adjust position to stay within container bounds', () => {
      // Position near right edge
      const data: TooltipData = {
        x: 750, // Near the 800px container width
        y: 100,
        time: '2024-01-15',
        price: 50000,
        visible: true
      };
      
      render(<ChartTooltip data={data} containerRef={mockContainerRef} />);
      const tooltip = screen.getByTestId('chart-tooltip');
      const rect = tooltip.getBoundingClientRect();
      
      // Tooltip should be positioned to the left of cursor when near right edge
      const expectedLeft = 750 - rect.width - 15; // x - width - offset
      expect(parseInt(tooltip.style.left)).toBeLessThan(750);
    });

    it('should handle positioning without container ref', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15',
        price: 50000,
        visible: true
      };
      
      render(<ChartTooltip data={data} />);
      const tooltip = screen.getByTestId('chart-tooltip');
      expect(tooltip).toHaveStyle({
        position: 'absolute',
        pointerEvents: 'none'
      });
    });
  });

  describe('Style Properties', () => {
    it('should have correct base styles', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15',
        price: 50000,
        visible: true
      };
      
      render(<ChartTooltip data={data} />);
      const tooltip = screen.getByTestId('chart-tooltip');
      
      expect(tooltip).toHaveStyle({
        position: 'absolute',
        pointerEvents: 'none',
        zIndex: 1000,
        borderRadius: '4px',
        minWidth: '160px'
      });
    });

    it('should have no transition for smooth following', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15',
        price: 50000,
        visible: true
      };
      
      render(<ChartTooltip data={data} />);
      const tooltip = screen.getByTestId('chart-tooltip');
      expect(tooltip).toHaveStyle({ transition: 'none' });
    });
  });

  describe('Update Behavior', () => {
    it('should update position when data changes', () => {
      const initialData: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15',
        price: 50000,
        visible: true
      };
      
      const { rerender } = render(<ChartTooltip data={initialData} />);
      const tooltip = screen.getByTestId('chart-tooltip');
      const initialLeft = parseInt(tooltip.style.left);
      const initialTop = parseInt(tooltip.style.top);
      expect(initialLeft).toBe(115); // 100 + 15 offset
      expect(initialTop).toBe(115); // 100 + 15 offset
      
      const updatedData: TooltipData = {
        ...initialData,
        x: 200,
        y: 300
      };
      
      rerender(<ChartTooltip data={updatedData} />);
      const newLeft = parseInt(tooltip.style.left);
      const newTop = parseInt(tooltip.style.top);
      expect(newLeft).toBe(215); // 200 + 15 offset
      expect(newTop).toBe(315); // 300 + 15 offset
    });

    it('should update content when data changes', () => {
      const initialData: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15 10:00:00',
        price: 50000,
        visible: true
      };
      
      const { rerender } = render(<ChartTooltip data={initialData} />);
      expect(screen.getByText('$50,000.00')).toBeInTheDocument();
      
      const updatedData: TooltipData = {
        ...initialData,
        price: 55000,
        time: '2024-01-15 11:00:00'
      };
      
      rerender(<ChartTooltip data={updatedData} />);
      expect(screen.getByText('$55,000.00')).toBeInTheDocument();
      expect(screen.getByText('2024-01-15 11:00:00')).toBeInTheDocument();
    });

    it('should hide when data becomes null', () => {
      const data: TooltipData = {
        x: 100,
        y: 100,
        time: '2024-01-15',
        price: 50000,
        visible: true
      };
      
      const { rerender, container } = render(<ChartTooltip data={data} />);
      expect(screen.getByTestId('chart-tooltip')).toBeInTheDocument();
      
      rerender(<ChartTooltip data={null} />);
      expect(container.querySelector('.chart-tooltip')).toBeNull();
    });
  });
});