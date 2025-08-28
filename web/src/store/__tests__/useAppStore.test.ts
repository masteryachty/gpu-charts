import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useAppStore, StoreState, StoreSubscriptionCallbacks } from '../useAppStore';

describe('useAppStore', () => {
  beforeEach(() => {
    // Reset the store to default state before each test
    const { getState, setState } = useAppStore;
    const defaultState = {
      symbol: 'coinbase:BTC-USD',
      startTime: Math.floor(Date.now() / 1000) - 24 * 60 * 60,
      endTime: Math.floor(Date.now() / 1000),
      preset: 'Market Data',
      isConnected: false,
      comparisonMode: false,
      selectedExchanges: ['coinbase'],
      baseSymbol: 'BTC-USD',
      _subscriptions: new Map(),
      _lastState: null
    };
    setState(defaultState as any);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should initialize with default state', () => {
    const { result } = renderHook(() => useAppStore());
    const state = result.current;

    expect(state.symbol).toBe('coinbase:BTC-USD');
    expect(state.preset).toBe('Market Data');
    expect(state.isConnected).toBe(false);
    expect(state.comparisonMode).toBe(false);
    expect(state.selectedExchanges).toEqual(['coinbase']);
    expect(state.baseSymbol).toBe('BTC-USD');
    expect(state.startTime).toBeTypeOf('number');
    expect(state.endTime).toBeTypeOf('number');
  });

  it('should update symbol correctly', () => {
    const { result } = renderHook(() => useAppStore());

    act(() => {
      result.current.setCurrentSymbol('kraken:ETH-USD');
    });

    expect(result.current.symbol).toBe('kraken:ETH-USD');
    expect(result.current.baseSymbol).toBe('ETH-USD');
    expect(result.current.selectedExchanges).toEqual(['kraken']);
  });

  it('should update preset correctly', () => {
    const { result } = renderHook(() => useAppStore());

    act(() => {
      result.current.setPreset('Order Book');
    });

    expect(result.current.preset).toBe('Order Book');
  });

  it('should update connection status', () => {
    const { result } = renderHook(() => useAppStore());

    act(() => {
      result.current.setIsConnected(true);
    });

    expect(result.current.isConnected).toBe(true);

    act(() => {
      result.current.setIsConnected(false);
    });

    expect(result.current.isConnected).toBe(false);
  });

  it('should update time range correctly', () => {
    const { result } = renderHook(() => useAppStore());
    const newStartTime = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago
    const newEndTime = Math.floor(Date.now() / 1000); // Now

    act(() => {
      result.current.setTimeRange(newStartTime, newEndTime);
    });

    expect(result.current.startTime).toBe(newStartTime);
    expect(result.current.endTime).toBe(newEndTime);
  });

  it('should handle batch updates', () => {
    const { result } = renderHook(() => useAppStore());
    const updates: Partial<StoreState> = {
      symbol: 'binance:ADA-USD',
      preset: 'Trades',
      isConnected: true
    };

    act(() => {
      result.current.updateChartState(updates);
    });

    expect(result.current.symbol).toBe('binance:ADA-USD');
    expect(result.current.preset).toBe('Trades');
    expect(result.current.isConnected).toBe(true);
  });

  it('should reset to defaults', () => {
    const { result } = renderHook(() => useAppStore());

    // Change some values first
    act(() => {
      result.current.setCurrentSymbol('kraken:ETH-USD');
      result.current.setPreset('Order Book');
      result.current.setIsConnected(true);
    });

    // Reset to defaults
    act(() => {
      result.current.resetToDefaults();
    });

    expect(result.current.symbol).toBe('coinbase:BTC-USD');
    expect(result.current.preset).toBe('Market Data');
    expect(result.current.isConnected).toBe(false);
    expect(result.current.comparisonMode).toBe(false);
  });

  it('should handle comparison mode', () => {
    const { result } = renderHook(() => useAppStore());

    act(() => {
      result.current.setComparisonMode(true);
    });

    expect(result.current.comparisonMode).toBe(true);

    act(() => {
      result.current.setComparisonMode(false);
    });

    expect(result.current.comparisonMode).toBe(false);
  });

  it('should handle exchange selection in normal mode', () => {
    const { result } = renderHook(() => useAppStore());

    act(() => {
      result.current.toggleExchange('kraken', 'ETH-USD');
    });

    expect(result.current.selectedExchanges).toEqual(['kraken:ETH-USD']);
    expect(result.current.symbol).toBe('kraken:ETH-USD');
  });

  it('should handle exchange selection in comparison mode', () => {
    const { result } = renderHook(() => useAppStore());

    // Enable comparison mode first
    act(() => {
      result.current.setComparisonMode(true);
    });

    // Add first exchange
    act(() => {
      result.current.toggleExchange('coinbase', 'BTC-USD');
    });

    // Add second exchange
    act(() => {
      result.current.toggleExchange('kraken', 'BTC-USD');
    });

    expect(result.current.selectedExchanges).toHaveLength(2);
    expect(result.current.selectedExchanges).toContain('coinbase:BTC-USD');
    expect(result.current.selectedExchanges).toContain('kraken:BTC-USD');
  });

  it('should limit exchanges to 2 in comparison mode', () => {
    const { result } = renderHook(() => useAppStore());

    // Enable comparison mode
    act(() => {
      result.current.setComparisonMode(true);
    });

    // Add three exchanges
    act(() => {
      result.current.toggleExchange('coinbase', 'BTC-USD');
      result.current.toggleExchange('kraken', 'BTC-USD');
      result.current.toggleExchange('binance', 'BTC-USD');
    });

    // Should only keep the first 2
    expect(result.current.selectedExchanges).toHaveLength(2);
  });

  it('should maintain at least one exchange', () => {
    const { result } = renderHook(() => useAppStore());

    // Enable comparison mode and set one exchange
    act(() => {
      result.current.setComparisonMode(true);
      result.current.setSelectedExchanges(['coinbase:BTC-USD']);
    });

    // Try to remove the last exchange
    act(() => {
      result.current.toggleExchange('coinbase', 'BTC-USD');
    });

    // Should still have one exchange
    expect(result.current.selectedExchanges).toHaveLength(1);
  });

  it('should update base symbol correctly', () => {
    const { result } = renderHook(() => useAppStore());

    act(() => {
      result.current.setBaseSymbol('ETH-USD');
    });

    expect(result.current.baseSymbol).toBe('ETH-USD');
    expect(result.current.symbol).toBe('coinbase:ETH-USD'); // Should update main symbol too in non-comparison mode
  });

  it('should handle subscriptions correctly', () => {
    const { result } = renderHook(() => useAppStore());
    const mockCallback = vi.fn();
    const callbacks: StoreSubscriptionCallbacks = {
      onSymbolChange: mockCallback
    };

    let unsubscribe: (() => void) | undefined;

    act(() => {
      unsubscribe = result.current.subscribe('test-subscription', callbacks);
    });

    // Change symbol to trigger subscription
    act(() => {
      result.current.setCurrentSymbol('kraken:ETH-USD');
    });

    expect(mockCallback).toHaveBeenCalledWith('kraken:ETH-USD', 'coinbase:BTC-USD');

    // Unsubscribe and change symbol again
    act(() => {
      unsubscribe?.();
      result.current.setCurrentSymbol('binance:ADA-USD');
    });

    // Should not be called again after unsubscribe
    expect(mockCallback).toHaveBeenCalledTimes(1);
  });

  it('should handle time range subscriptions', () => {
    const { result } = renderHook(() => useAppStore());
    const mockTimeRangeCallback = vi.fn();
    const callbacks: StoreSubscriptionCallbacks = {
      onTimeRangeChange: mockTimeRangeCallback
    };

    act(() => {
      result.current.subscribe('time-subscription', callbacks);
    });

    const newStart = Math.floor(Date.now() / 1000) - 7200; // 2 hours ago
    const newEnd = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago

    act(() => {
      result.current.setTimeRange(newStart, newEnd);
    });

    expect(mockTimeRangeCallback).toHaveBeenCalledWith(
      { startTime: newStart, endTime: newEnd },
      expect.objectContaining({ startTime: expect.any(Number), endTime: expect.any(Number) })
    );
  });

  it('should handle preset subscriptions', () => {
    const { result } = renderHook(() => useAppStore());
    const mockPresetCallback = vi.fn();
    const callbacks: StoreSubscriptionCallbacks = {
      onPresetChange: mockPresetCallback
    };

    act(() => {
      result.current.subscribe('preset-subscription', callbacks);
    });

    act(() => {
      result.current.setPreset('Order Book');
    });

    expect(mockPresetCallback).toHaveBeenCalledWith('Order Book', 'Market Data');
  });

  it('should handle general change subscriptions', () => {
    const { result } = renderHook(() => useAppStore());
    const mockAnyChangeCallback = vi.fn();
    const callbacks: StoreSubscriptionCallbacks = {
      onAnyChange: mockAnyChangeCallback
    };

    act(() => {
      result.current.subscribe('any-change-subscription', callbacks);
    });

    act(() => {
      result.current.setCurrentSymbol('kraken:ETH-USD');
    });

    expect(mockAnyChangeCallback).toHaveBeenCalledWith(
      expect.objectContaining({ symbol: 'kraken:ETH-USD' }),
      expect.objectContaining({ symbol: 'coinbase:BTC-USD' })
    );
  });

  it('should handle multiple subscriptions', () => {
    const { result } = renderHook(() => useAppStore());
    const callback1 = vi.fn();
    const callback2 = vi.fn();

    act(() => {
      result.current.subscribe('sub1', { onSymbolChange: callback1 });
      result.current.subscribe('sub2', { onSymbolChange: callback2 });
    });

    act(() => {
      result.current.setCurrentSymbol('kraken:ETH-USD');
    });

    expect(callback1).toHaveBeenCalledTimes(1);
    expect(callback2).toHaveBeenCalledTimes(1);
  });

  it('should handle unsubscribe by ID', () => {
    const { result } = renderHook(() => useAppStore());
    const callback = vi.fn();

    act(() => {
      result.current.subscribe('test-sub', { onSymbolChange: callback });
      result.current.setCurrentSymbol('kraken:ETH-USD');
    });

    expect(callback).toHaveBeenCalledTimes(1);

    act(() => {
      result.current.unsubscribe('test-sub');
      result.current.setCurrentSymbol('binance:ADA-USD');
    });

    // Should not be called after unsubscribe
    expect(callback).toHaveBeenCalledTimes(1);
  });

  it('should extract exchange and symbol from compound symbol', () => {
    const { result } = renderHook(() => useAppStore());

    act(() => {
      result.current.setCurrentSymbol('binance:SOL-USD');
    });

    expect(result.current.symbol).toBe('binance:SOL-USD');
    expect(result.current.baseSymbol).toBe('SOL-USD');
    expect(result.current.selectedExchanges).toEqual(['binance']);
  });

  it('should handle symbol without exchange prefix', () => {
    const { result } = renderHook(() => useAppStore());

    act(() => {
      result.current.setCurrentSymbol('BTC-USD');
    });

    expect(result.current.symbol).toBe('BTC-USD');
    expect(result.current.baseSymbol).toBe('BTC-USD');
    // selectedExchanges should remain unchanged when no exchange prefix
  });
});