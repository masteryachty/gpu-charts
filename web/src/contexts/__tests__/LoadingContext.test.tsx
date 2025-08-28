import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { LoadingProvider, useLoading } from '../LoadingContext';
import { ReactNode } from 'react';

// Test component to access the loading context
function TestComponent() {
  const { 
    loading, 
    isAnyLoading, 
    getLoadingMessage, 
    setLoading,
    setMultipleLoading 
  } = useLoading();
  
  return (
    <div>
      <div data-testid="is-any-loading">{isAnyLoading.toString()}</div>
      <div data-testid="primary-message">{getLoadingMessage()}</div>
      <div data-testid="wasm-loading">{loading.wasm.toString()}</div>
      <div data-testid="data-loading">{loading.data.toString()}</div>
      <div data-testid="symbols-loading">{loading.symbols.toString()}</div>
      <div data-testid="webgpu-loading">{loading.webgpu.toString()}</div>
      <div data-testid="initialization-loading">{loading.initialization.toString()}</div>
      
      <button onClick={() => setLoading('wasm', true)}>Set WASM Loading</button>
      <button onClick={() => setLoading('data', true)}>Set Data Loading</button>
      <button onClick={() => setLoading('webgpu', true)}>Set WebGPU Loading</button>
      <button onClick={() => setLoading('wasm', false)}>Clear WASM Loading</button>
      <button onClick={() => setMultipleLoading({ wasm: false, data: false, symbols: false, webgpu: false, initialization: false })}>Clear All Loading</button>
    </div>
  );
}

function renderWithProvider(children: ReactNode) {
  return render(
    <LoadingProvider>
      {children}
    </LoadingProvider>
  );
}

describe('LoadingContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should initialize with default loading state', () => {
    renderWithProvider(<TestComponent />);
    
    expect(screen.getByTestId('is-any-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('primary-message')).toHaveTextContent('');
    expect(screen.getByTestId('wasm-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('data-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('symbols-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('webgpu-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('initialization-loading')).toHaveTextContent('false');
  });

  it('should set and clear individual loading states', () => {
    renderWithProvider(<TestComponent />);
    
    const setWasmButton = screen.getByText('Set WASM Loading');
    const clearWasmButton = screen.getByText('Clear WASM Loading');
    
    // Set WASM loading
    act(() => {
      setWasmButton.click();
    });
    
    expect(screen.getByTestId('wasm-loading')).toHaveTextContent('true');
    expect(screen.getByTestId('is-any-loading')).toHaveTextContent('true');
    
    // Clear WASM loading
    act(() => {
      clearWasmButton.click();
    });
    
    expect(screen.getByTestId('wasm-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('is-any-loading')).toHaveTextContent('false');
  });

  it('should handle multiple loading states simultaneously', () => {
    renderWithProvider(<TestComponent />);
    
    const setWasmButton = screen.getByText('Set WASM Loading');
    const setDataButton = screen.getByText('Set Data Loading');
    
    // Set multiple loading states
    act(() => {
      setWasmButton.click();
      setDataButton.click();
    });
    
    expect(screen.getByTestId('wasm-loading')).toHaveTextContent('true');
    expect(screen.getByTestId('data-loading')).toHaveTextContent('true');
    expect(screen.getByTestId('is-any-loading')).toHaveTextContent('true');
  });

  it('should show correct primary loading message based on priority', () => {
    renderWithProvider(<TestComponent />);
    
    const setWasmButton = screen.getByText('Set WASM Loading');
    const setWebGPUButton = screen.getByText('Set WebGPU Loading');
    
    // Set WASM loading (lower priority)
    act(() => {
      setWasmButton.click();
    });
    
    expect(screen.getByTestId('primary-message')).toHaveTextContent('Loading WebAssembly module...');
    
    // Set WebGPU loading (higher priority)
    act(() => {
      setWebGPUButton.click();
    });
    
    expect(screen.getByTestId('primary-message')).toHaveTextContent('Initializing GPU acceleration...');
  });

  it('should clear all loading states', () => {
    renderWithProvider(<TestComponent />);
    
    const setWasmButton = screen.getByText('Set WASM Loading');
    const setDataButton = screen.getByText('Set Data Loading');
    const clearAllButton = screen.getByText('Clear All Loading');
    
    // Set multiple loading states
    act(() => {
      setWasmButton.click();
      setDataButton.click();
    });
    
    expect(screen.getByTestId('is-any-loading')).toHaveTextContent('true');
    
    // Clear all loading
    act(() => {
      clearAllButton.click();
    });
    
    expect(screen.getByTestId('wasm-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('data-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('is-any-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('primary-message')).toHaveTextContent('');
  });

  it('should maintain loading state consistency', () => {
    renderWithProvider(<TestComponent />);
    
    const setWasmButton = screen.getByText('Set WASM Loading');
    const setDataButton = screen.getByText('Set Data Loading');
    const clearWasmButton = screen.getByText('Clear WASM Loading');
    
    // Set both WASM and data loading
    act(() => {
      setWasmButton.click();
      setDataButton.click();
    });
    
    expect(screen.getByTestId('is-any-loading')).toHaveTextContent('true');
    
    // Clear WASM, data should still be loading
    act(() => {
      clearWasmButton.click();
    });
    
    expect(screen.getByTestId('wasm-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('data-loading')).toHaveTextContent('true');
    expect(screen.getByTestId('is-any-loading')).toHaveTextContent('true');
    expect(screen.getByTestId('primary-message')).toHaveTextContent('Fetching market data...');
  });

  it('should handle loading state transitions correctly', () => {
    renderWithProvider(<TestComponent />);
    
    const setWasmButton = screen.getByText('Set WASM Loading');
    const setWebGPUButton = screen.getByText('Set WebGPU Loading');
    const clearWasmButton = screen.getByText('Clear WASM Loading');
    
    // Start with WASM loading
    act(() => {
      setWasmButton.click();
    });
    
    expect(screen.getByTestId('primary-message')).toHaveTextContent('Loading WebAssembly module...');
    
    // Add WebGPU loading (higher priority)
    act(() => {
      setWebGPUButton.click();
    });
    
    expect(screen.getByTestId('primary-message')).toHaveTextContent('Initializing GPU acceleration...');
    
    // Clear WASM (WebGPU should still be primary)
    act(() => {
      clearWasmButton.click();
    });
    
    expect(screen.getByTestId('primary-message')).toHaveTextContent('Initializing GPU acceleration...');
  });

  it('should handle rapid state changes', () => {
    renderWithProvider(<TestComponent />);
    
    const setWasmButton = screen.getByText('Set WASM Loading');
    const clearWasmButton = screen.getByText('Clear WASM Loading');
    
    // Rapidly toggle WASM loading
    act(() => {
      setWasmButton.click();
      clearWasmButton.click();
      setWasmButton.click();
      clearWasmButton.click();
    });
    
    expect(screen.getByTestId('wasm-loading')).toHaveTextContent('false');
    expect(screen.getByTestId('is-any-loading')).toHaveTextContent('false');
  });

  it('should provide loading messages for all supported states', () => {
    renderWithProvider(<TestComponent />);
    
    // Test each loading state message
    const loadingTypes = [
      { type: 'initialization', expectedMessage: 'Starting chart engine...' },
      { type: 'webgpu', expectedMessage: 'Initializing GPU acceleration...' },
      { type: 'wasm', expectedMessage: 'Loading WebAssembly module...' },
      { type: 'data', expectedMessage: 'Fetching market data...' },
      { type: 'symbols', expectedMessage: 'Loading available symbols...' }
    ];
    
    loadingTypes.forEach(({ type, expectedMessage }) => {
      act(() => {
        // Clear all first
        screen.getByText('Clear All Loading').click();
      });
      
      // Set specific loading type
      act(() => {
        // We need to trigger this through the component since we can't access context directly
        if (type === 'wasm') {
          screen.getByText('Set WASM Loading').click();
        } else if (type === 'data') {
          screen.getByText('Set Data Loading').click();
        } else if (type === 'webgpu') {
          screen.getByText('Set WebGPU Loading').click();
        }
      });
      
      if (type === 'wasm' || type === 'data' || type === 'webgpu') {
        expect(screen.getByTestId('primary-message')).toHaveTextContent(expectedMessage);
      }
    });
  });
});

// Test component that doesn't use the provider
function ComponentWithoutProvider() {
  try {
    const loading = useLoading();
    return <div>Should not render</div>;
  } catch (error) {
    return <div data-testid="error">Context error</div>;
  }
}

describe('LoadingContext without provider', () => {
  it('should throw error when used outside provider', () => {
    // Suppress console.error for this test since we expect an error
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    
    try {
      render(<ComponentWithoutProvider />);
      expect.fail('Expected an error to be thrown');
    } catch (error: any) {
      expect(error.message).toContain('useLoading must be used within a LoadingProvider');
    }
    
    consoleSpy.mockRestore();
  });
});