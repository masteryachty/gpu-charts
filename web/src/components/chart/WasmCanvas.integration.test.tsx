import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import WasmCanvas from './WasmCanvas';
import { useWasmChart } from '../../hooks/useWasmChart';
import { useAppStore } from '../../store/useAppStore';

// Mock the hooks
vi.mock('../../hooks/useWasmChart');
vi.mock('../../store/useAppStore');

describe('WasmCanvas Tooltip Integration', () => {
  let mockChart: any;
  let mockChartState: any;
  let mockChartAPI: any;

  beforeEach(() => {
    // Mock chart instance with tooltip-related methods
    mockChart = {
      handle_mouse_move: vi.fn(),
      handle_mouse_click: vi.fn(),
      handle_mouse_right_click: vi.fn(),
      handle_mouse_wheel: vi.fn(),
      get_tooltip_data: vi.fn(),
      update_time_range: vi.fn().mockResolvedValue(undefined),
    };

    mockChartState = {
      isInitialized: true,
      chart: mockChart,
      error: null,
    };

    mockChartAPI = {
      initialize: vi.fn().mockResolvedValue(true),
      cleanup: vi.fn(),
    };

    // Mock the hooks
    (useWasmChart as any).mockReturnValue([mockChartState, mockChartAPI]);
    (useAppStore as any).mockReturnValue({
      startTime: 1700000000,
      endTime: 1700086400,
    });

    // Mock canvas getBoundingClientRect
    HTMLCanvasElement.prototype.getBoundingClientRect = vi.fn(() => ({
      left: 0,
      top: 0,
      right: 800,
      bottom: 600,
      width: 800,
      height: 600,
      x: 0,
      y: 0,
      toJSON: () => {},
    }));
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Tooltip Display on Right-Click', () => {
    it('should show tooltip when right-clicking and holding', async () => {
      // Mock tooltip data
      mockChart.get_tooltip_data.mockReturnValue({
        time: '2024-01-15 10:30:00',
        price: 50000,
        volume: 1234.5,
        side: 'buy',
        best_bid: 49999,
        best_ask: 50001,
      });

      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      // Right-click on canvas
      fireEvent.mouseDown(canvas, { 
        button: 2, 
        clientX: 400, 
        clientY: 300 
      });

      // Wait for tooltip to appear
      await waitFor(() => {
        expect(screen.getByTestId('chart-tooltip')).toBeInTheDocument();
      });

      // Verify tooltip content
      expect(screen.getByText('2024-01-15 10:30:00')).toBeInTheDocument();
      expect(screen.getByText('$50,000.00')).toBeInTheDocument();
      expect(screen.getByText('1.23K')).toBeInTheDocument();
      expect(screen.getByText('BUY')).toBeInTheDocument();

      // Verify WASM methods were called
      expect(mockChart.get_tooltip_data).toHaveBeenCalledWith(400, 300);
      expect(mockChart.handle_mouse_right_click).toHaveBeenCalledWith(400, 300, true);
    });

    it('should hide tooltip when releasing right-click', async () => {
      mockChart.get_tooltip_data.mockReturnValue({
        time: '2024-01-15 10:30:00',
        price: 50000,
      });

      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      // Right-click to show tooltip
      fireEvent.mouseDown(canvas, { 
        button: 2, 
        clientX: 400, 
        clientY: 300 
      });

      await waitFor(() => {
        expect(screen.getByTestId('chart-tooltip')).toBeInTheDocument();
      });

      // Release right-click
      fireEvent.mouseUp(canvas, { 
        button: 2, 
        clientX: 400, 
        clientY: 300 
      });

      // Tooltip should disappear
      await waitFor(() => {
        expect(screen.queryByTestId('chart-tooltip')).not.toBeInTheDocument();
      });

      expect(mockChart.handle_mouse_right_click).toHaveBeenCalledWith(400, 300, false);
    });

    it('should use fallback data when get_tooltip_data is not available', async () => {
      // Remove the get_tooltip_data method
      delete mockChart.get_tooltip_data;

      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      // Right-click on canvas
      fireEvent.mouseDown(canvas, { 
        button: 2, 
        clientX: 400, 
        clientY: 300 
      });

      // Tooltip should appear with fallback data
      await waitFor(() => {
        expect(screen.getByTestId('chart-tooltip')).toBeInTheDocument();
      });

      // Should have price but with fallback random value
      expect(screen.getByText(/\$[\d,]+\.\d{2}/)).toBeInTheDocument();
    });

    it('should handle errors gracefully when getting tooltip data', async () => {
      mockChart.get_tooltip_data.mockImplementation(() => {
        throw new Error('Failed to get data');
      });

      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      // Right-click on canvas
      fireEvent.mouseDown(canvas, { 
        button: 2, 
        clientX: 400, 
        clientY: 300 
      });

      // Tooltip should still appear with fallback data
      await waitFor(() => {
        expect(screen.getByTestId('chart-tooltip')).toBeInTheDocument();
      });

      expect(consoleSpy).toHaveBeenCalledWith(
        '[WasmCanvas] Error getting tooltip data:',
        expect.any(Error)
      );

      consoleSpy.mockRestore();
    });
  });

  describe('Tooltip Movement', () => {
    it('should update tooltip position when mouse moves while holding right-click', async () => {
      mockChart.get_tooltip_data
        .mockReturnValueOnce({
          time: '2024-01-15 10:30:00',
          price: 50000,
        })
        .mockReturnValueOnce({
          time: '2024-01-15 10:30:01',
          price: 50100,
        });

      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      // Right-click at initial position
      fireEvent.mouseDown(canvas, { 
        button: 2, 
        clientX: 200, 
        clientY: 200 
      });

      await waitFor(() => {
        expect(screen.getByTestId('chart-tooltip')).toBeInTheDocument();
      });

      const tooltip = screen.getByTestId('chart-tooltip');
      const initialLeft = parseInt(tooltip.style.left);
      const initialTop = parseInt(tooltip.style.top);

      // Move mouse while holding right-click
      fireEvent.mouseMove(canvas, { 
        clientX: 400, 
        clientY: 300 
      });

      await waitFor(() => {
        const newLeft = parseInt(tooltip.style.left);
        const newTop = parseInt(tooltip.style.top);
        expect(newLeft).not.toBe(initialLeft);
        expect(newTop).not.toBe(initialTop);
      });

      // Verify updated content
      expect(screen.getByText('$50,100.00')).toBeInTheDocument();
    });

    it('should not show tooltip on regular mouse move without right-click', () => {
      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      // Move mouse without clicking
      fireEvent.mouseMove(canvas, { 
        clientX: 400, 
        clientY: 300 
      });

      // Tooltip should not appear
      expect(screen.queryByTestId('chart-tooltip')).not.toBeInTheDocument();
      expect(mockChart.handle_mouse_move).toHaveBeenCalledWith(400, 300);
    });
  });

  describe('Context Menu Prevention', () => {
    it('should prevent context menu on right-click', () => {
      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      const contextMenuEvent = new MouseEvent('contextmenu', {
        bubbles: true,
        cancelable: true,
        clientX: 400,
        clientY: 300,
      });

      const preventDefault = vi.spyOn(contextMenuEvent, 'preventDefault');
      fireEvent(canvas, contextMenuEvent);

      expect(preventDefault).toHaveBeenCalled();
    });
  });

  describe('Interaction with Other Mouse Events', () => {
    it('should not interfere with left-click drag operations', () => {
      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      // Left-click for drag
      fireEvent.mouseDown(canvas, { 
        button: 0, 
        clientX: 400, 
        clientY: 300 
      });

      // Should not show tooltip
      expect(screen.queryByTestId('chart-tooltip')).not.toBeInTheDocument();
      
      // Should call drag handler
      expect(mockChart.handle_mouse_click).toHaveBeenCalledWith(400, 300, true);

      // Release left-click
      fireEvent.mouseUp(canvas, { 
        button: 0, 
        clientX: 500, 
        clientY: 300 
      });

      expect(mockChart.handle_mouse_click).toHaveBeenCalledWith(500, 300, false);
    });

    it('should not interfere with mouse wheel zoom', () => {
      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      const wheelEvent = new WheelEvent('wheel', {
        bubbles: true,
        cancelable: true,
        clientX: 400,
        clientY: 300,
        deltaY: 100,
      });

      fireEvent(canvas, wheelEvent);

      expect(mockChart.handle_mouse_wheel).toHaveBeenCalledWith(100, 400, 300);
      expect(screen.queryByTestId('chart-tooltip')).not.toBeInTheDocument();
    });
  });

  describe('Tooltip with Uninitialized Chart', () => {
    it('should not show tooltip when chart is not initialized', () => {
      // Set chart as uninitialized
      (useWasmChart as any).mockReturnValue([
        { isInitialized: false, chart: null, error: null },
        mockChartAPI
      ]);

      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      // Try to right-click
      fireEvent.mouseDown(canvas, { 
        button: 2, 
        clientX: 400, 
        clientY: 300 
      });

      // Tooltip should not appear
      expect(screen.queryByTestId('chart-tooltip')).not.toBeInTheDocument();
    });
  });

  describe('Tooltip Data Updates', () => {
    it('should update tooltip with all available data fields', async () => {
      mockChart.get_tooltip_data.mockReturnValue({
        time: '2024-01-15 10:30:00',
        price: 50000,
        volume: 1234567,
        side: 'sell',
        best_bid: 49999.50,
        best_ask: 50000.50,
      });

      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      fireEvent.mouseDown(canvas, { 
        button: 2, 
        clientX: 400, 
        clientY: 300 
      });

      await waitFor(() => {
        expect(screen.getByTestId('chart-tooltip')).toBeInTheDocument();
      });

      // Verify all fields are displayed
      expect(screen.getByText('Time:')).toBeInTheDocument();
      expect(screen.getByText('Price:')).toBeInTheDocument();
      expect(screen.getByText('Volume:')).toBeInTheDocument();
      expect(screen.getByText('Side:')).toBeInTheDocument();
      expect(screen.getByText('Bid:')).toBeInTheDocument();
      expect(screen.getByText('Ask:')).toBeInTheDocument();
      
      // Verify formatted values
      expect(screen.getByText('1.23M')).toBeInTheDocument(); // Volume
      expect(screen.getByText('SELL')).toBeInTheDocument(); // Side
      expect(screen.getByText('$49,999.50')).toBeInTheDocument(); // Bid
      expect(screen.getByText('$50,000.50')).toBeInTheDocument(); // Ask
    });

    it('should handle partial data gracefully', async () => {
      // Only return minimal data
      mockChart.get_tooltip_data.mockReturnValue({
        time: '2024-01-15 10:30:00',
        price: 50000,
      });

      render(<WasmCanvas />);
      const canvas = screen.getByTestId('wasm-canvas');

      fireEvent.mouseDown(canvas, { 
        button: 2, 
        clientX: 400, 
        clientY: 300 
      });

      await waitFor(() => {
        expect(screen.getByTestId('chart-tooltip')).toBeInTheDocument();
      });

      // Should show time and price
      expect(screen.getByText('Time:')).toBeInTheDocument();
      expect(screen.getByText('Price:')).toBeInTheDocument();
      
      // Should not show optional fields
      expect(screen.queryByText('Volume:')).not.toBeInTheDocument();
      expect(screen.queryByText('Side:')).not.toBeInTheDocument();
      expect(screen.queryByText('Bid:')).not.toBeInTheDocument();
      expect(screen.queryByText('Ask:')).not.toBeInTheDocument();
    });
  });
});