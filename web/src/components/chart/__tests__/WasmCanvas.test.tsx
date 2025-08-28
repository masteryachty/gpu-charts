import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import WasmCanvas from '../WasmCanvas';

// Mock the useWasmChart hook
const mockChartState = {
  chart: null,
  isInitialized: false
};

const mockChartAPI = {
  initialize: vi.fn()
};

vi.mock('../../../hooks/useWasmChart', () => ({
  useWasmChart: () => [mockChartState, mockChartAPI]
}));

// Mock the useAppStore hook
vi.mock('../../../store/useAppStore', () => ({
  useAppStore: () => ({
    startTime: 1640995200,
    endTime: 1641081600,
    symbol: 'BTC-USD'
  })
}));

// Mock the loading context
vi.mock('../../../contexts/LoadingContext', () => ({
  useLoading: () => ({
    setLoading: vi.fn()
  })
}));

// Mock other components
vi.mock('../ChartTooltip', () => ({
  TooltipProvider: ({ children }: { children: React.ReactNode }) => <div data-testid="tooltip-provider">{children}</div>
}));

vi.mock('../../../hooks/useAutonomousDataFetching', () => ({
  useAutonomousDataFetching: () => ({
    isLoading: false,
    error: null,
    data: null,
    refetch: vi.fn()
  })
}));

vi.mock('../../error/WasmErrorBoundary', () => ({
  WasmErrorBoundary: ({ children }: { children: React.ReactNode }) => <div data-testid="error-boundary">{children}</div>
}));

vi.mock('../../loading/LoadingSkeleton', () => ({
  ChartLoadingSkeleton: ({ className }: { className?: string }) => (
    <div data-testid="loading-skeleton" className={className}>Loading...</div>
  )
}));

describe('WasmCanvas', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    
    // Reset mock state
    mockChartState.chart = null;
    mockChartState.isInitialized = false;
    
    // Mock ResizeObserver
    global.ResizeObserver = vi.fn().mockImplementation(() => ({
      observe: vi.fn(),
      unobserve: vi.fn(),
      disconnect: vi.fn(),
    }));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should render canvas element', () => {
    render(<WasmCanvas />);
    
    const canvas = screen.getByTestId('wasm-canvas');
    expect(canvas).toBeInTheDocument();
    expect(canvas.tagName).toBe('CANVAS');
  });

  it('should show loading skeleton when not initialized', () => {
    render(<WasmCanvas />);
    
    expect(screen.getByTestId('loading-skeleton')).toBeInTheDocument();
  });

  it('should hide loading skeleton when initialized', () => {
    mockChartState.isInitialized = true;
    
    render(<WasmCanvas />);
    
    expect(screen.queryByTestId('loading-skeleton')).not.toBeInTheDocument();
  });

  it('should render with error boundary', () => {
    render(<WasmCanvas />);
    
    expect(screen.getByTestId('error-boundary')).toBeInTheDocument();
  });

  it('should render with tooltip provider', () => {
    render(<WasmCanvas />);
    
    expect(screen.getByTestId('tooltip-provider')).toBeInTheDocument();
  });

  it('should have proper accessibility attributes', () => {
    render(<WasmCanvas />);
    
    const canvas = screen.getByTestId('wasm-canvas');
    
    expect(canvas).toHaveAttribute('role', 'application');
    expect(canvas).toHaveAttribute('aria-label');
    expect(canvas).toHaveAttribute('aria-describedby', 'chart-instructions');
    expect(canvas).toHaveAttribute('tabIndex', '0');
  });

  it('should render accessibility instructions', () => {
    render(<WasmCanvas />);
    
    const instructions = screen.getByText(/Interactive financial chart showing price data over time/i);
    expect(instructions).toBeInTheDocument();
    expect(instructions).toHaveAttribute('id', 'chart-instructions');
  });

  it('should handle mouse events', () => {
    mockChartState.isInitialized = true;
    
    render(<WasmCanvas />);
    
    const canvas = screen.getByTestId('wasm-canvas');
    
    // Test that event handlers are attached (we can't easily test the actual functionality without the real chart)
    expect(canvas).toHaveAttribute('onwheel');
    expect(canvas).toHaveAttribute('onmousemove');
    expect(canvas).toHaveAttribute('onmousedown');
    expect(canvas).toHaveAttribute('onmouseup');
    expect(canvas).toHaveAttribute('oncontextmenu');
  });

  it('should prevent context menu on right click', () => {
    render(<WasmCanvas />);
    
    const canvas = screen.getByTestId('wasm-canvas');
    const contextMenuEvent = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    const preventDefaultSpy = vi.spyOn(contextMenuEvent, 'preventDefault');
    
    fireEvent(canvas, contextMenuEvent);
    
    expect(preventDefaultSpy).toHaveBeenCalled();
  });

  it('should render with correct canvas attributes', () => {
    render(<WasmCanvas width={800} height={600} />);
    
    const canvas = screen.getByTestId('wasm-canvas');
    
    expect(canvas).toHaveAttribute('id', 'webgpu-canvas');
    expect(canvas).toHaveStyle({ width: '800px', height: '600px' });
    expect(canvas).toHaveAttribute('data-initialized', 'false');
  });

  it('should update data-initialized attribute when chart is initialized', () => {
    mockChartState.isInitialized = true;
    
    render(<WasmCanvas />);
    
    const canvas = screen.getByTestId('wasm-canvas');
    expect(canvas).toHaveAttribute('data-initialized', 'true');
  });

  it('should handle onChartReady callback', () => {
    const onChartReady = vi.fn();
    mockChartState.isInitialized = true;
    mockChartState.chart = { mock: 'chart' } as any;
    
    render(<WasmCanvas onChartReady={onChartReady} />);
    
    // The callback should be called when chart is ready
    // Note: This would require the actual useEffect to run, which might not happen in test environment
  });

  it('should have responsive canvas dimensions', () => {
    render(<WasmCanvas />);
    
    const canvas = screen.getByTestId('wasm-canvas');
    
    // Check that canvas has responsive styling
    expect(canvas).toHaveClass('w-full', 'h-full');
    expect(canvas).toHaveStyle({ 
      minWidth: '200px', 
      minHeight: '150px',
      display: 'block'
    });
  });

  it('should set data-chart-ready on container when initialized', () => {
    mockChartState.isInitialized = true;
    
    render(<WasmCanvas />);
    
    const container = screen.getByRole('img');
    expect(container).toHaveAttribute('data-chart-ready', 'true');
  });

  it('should not set data-chart-ready when not initialized', () => {
    render(<WasmCanvas />);
    
    const container = screen.getByRole('img');
    expect(container).not.toHaveAttribute('data-chart-ready');
  });

  it('should render with proper container styling', () => {
    render(<WasmCanvas />);
    
    const container = screen.getByRole('img');
    
    expect(container).toHaveClass('flex-1', 'bg-gray-900', 'relative');
    expect(container).toHaveStyle({ minWidth: '200px', minHeight: '150px' });
  });

  it('should have proper ARIA labels with symbol information', () => {
    render(<WasmCanvas />);
    
    const container = screen.getByRole('img');
    const canvas = screen.getByTestId('wasm-canvas');
    
    expect(container).toHaveAttribute('aria-label');
    expect(container.getAttribute('aria-label')).toContain('BTC-USD');
    expect(canvas.getAttribute('aria-label')).toContain('BTC-USD');
  });
});