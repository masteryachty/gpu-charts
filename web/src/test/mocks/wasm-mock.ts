// Mock WebAssembly module for testing
// This file replaces the actual WASM module during testing to avoid WebGPU dependencies

export interface MockWasmChart {
  initialize: (canvas: HTMLCanvasElement, config: any) => Promise<void>;
  set_data: (data: Uint8Array) => void;
  set_view_range: (start_time: number, end_time: number) => void;
  set_chart_type: (chart_type: string) => void;
  set_quality_preset: (preset: string) => void;
  handle_mouse_event: (event_type: string, x: number, y: number, button?: number) => any;
  handle_keyboard_event: (event_type: string, key: string) => any;
  handle_wheel_event: (delta_x: number, delta_y: number, x: number, y: number) => any;
  resize: (width: number, height: number) => void;
  render: () => void;
  get_tooltip_data: (x: number, y: number) => any;
  set_time_range_selection: (start: number, end: number) => void;
  clear_time_range_selection: () => void;
  get_visible_range: () => { start: number; end: number };
  get_data_bounds: () => { min_price: number; max_price: number; min_time: number; max_time: number };
  set_theme: (theme: string) => void;
  get_performance_stats: () => { fps: number; render_time: number; data_points: number };
  dispose: () => void;
}

export interface MockWasmModule {
  ChartRenderer: new () => MockWasmChart;
  initialize_logging: () => void;
  get_version: () => string;
}

// Mock chart instance with realistic behavior
class MockChartRenderer implements MockWasmChart {
  private canvas: HTMLCanvasElement | null = null;
  private config: any = {};
  private data: Uint8Array = new Uint8Array();
  private viewRange = { start: 0, end: 0 };
  private chartType = 'line';
  private qualityPreset = 'medium';
  private theme = 'dark';
  private isInitialized = false;
  private dataBounds = { min_price: 0, max_price: 100, min_time: 0, max_time: 1000 };

  async initialize(canvas: HTMLCanvasElement, config: any): Promise<void> {
    this.canvas = canvas;
    this.config = config;
    
    // Simulate initialization delay
    await new Promise(resolve => setTimeout(resolve, 100));
    
    this.isInitialized = true;
    
    // Trigger resize to simulate real initialization
    if (canvas.width && canvas.height) {
      this.resize(canvas.width, canvas.height);
    }
  }

  set_data(data: Uint8Array): void {
    this.data = data;
    
    // Simulate data bounds calculation
    if (data.length > 0) {
      this.dataBounds = {
        min_price: 45000 - Math.random() * 5000,
        max_price: 45000 + Math.random() * 5000,
        min_time: Date.now() - 86400000,
        max_time: Date.now()
      };
    }
  }

  set_view_range(start_time: number, end_time: number): void {
    this.viewRange = { start: start_time, end: end_time };
  }

  set_chart_type(chart_type: string): void {
    this.chartType = chart_type;
  }

  set_quality_preset(preset: string): void {
    this.qualityPreset = preset;
  }

  handle_mouse_event(event_type: string, x: number, y: number, button?: number): any {
    if (event_type === 'move' && this.isInitialized) {
      return {
        tooltip_data: {
          x,
          y,
          timestamp: this.viewRange.start + (x / (this.canvas?.width || 800)) * (this.viewRange.end - this.viewRange.start),
          price: 45000 + Math.random() * 1000,
          volume: Math.random() * 10000,
          exchange: 'coinbase',
          symbol: 'BTC-USD',
          change24h: (Math.random() - 0.5) * 0.1
        }
      };
    }
    return null;
  }

  handle_keyboard_event(event_type: string, key: string): any {
    // Mock keyboard handling
    return { handled: true };
  }

  handle_wheel_event(delta_x: number, delta_y: number, x: number, y: number): any {
    // Mock zoom/pan handling
    const zoomFactor = delta_y > 0 ? 1.1 : 0.9;
    const range = this.viewRange.end - this.viewRange.start;
    const newRange = range * zoomFactor;
    const center = this.viewRange.start + range * (x / (this.canvas?.width || 800));
    
    this.viewRange = {
      start: center - newRange / 2,
      end: center + newRange / 2
    };
    
    return { updated: true };
  }

  resize(width: number, height: number): void {
    if (this.canvas) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
  }

  render(): void {
    if (!this.canvas || !this.isInitialized) return;
    
    // Mock rendering with canvas 2D context
    const ctx = this.canvas.getContext('2d');
    if (!ctx) return;
    
    // Clear canvas
    ctx.fillStyle = this.theme === 'dark' ? '#1a1a1a' : '#ffffff';
    ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    
    // Draw mock chart line
    if (this.data.length > 0) {
      ctx.strokeStyle = '#3b82f6';
      ctx.lineWidth = 2;
      ctx.beginPath();
      
      const points = 100;
      for (let i = 0; i < points; i++) {
        const x = (i / (points - 1)) * this.canvas.width;
        const y = this.canvas.height / 2 + Math.sin(i * 0.1) * 50 + Math.random() * 20;
        
        if (i === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }
      
      ctx.stroke();
    }
  }

  get_tooltip_data(x: number, y: number): any {
    if (!this.isInitialized) return null;
    
    return {
      x,
      y,
      timestamp: this.viewRange.start + (x / (this.canvas?.width || 800)) * (this.viewRange.end - this.viewRange.start),
      price: 45000 + Math.random() * 1000,
      volume: Math.random() * 10000,
      exchange: 'coinbase',
      symbol: 'BTC-USD',
      change24h: (Math.random() - 0.5) * 0.1
    };
  }

  set_time_range_selection(start: number, end: number): void {
    // Mock time range selection
  }

  clear_time_range_selection(): void {
    // Mock clear selection
  }

  get_visible_range(): { start: number; end: number } {
    return this.viewRange;
  }

  get_data_bounds(): { min_price: number; max_price: number; min_time: number; max_time: number } {
    return this.dataBounds;
  }

  set_theme(theme: string): void {
    this.theme = theme;
  }

  get_performance_stats(): { fps: number; render_time: number; data_points: number } {
    return {
      fps: 60 + Math.random() * 5,
      render_time: 2 + Math.random() * 3,
      data_points: this.data.length / 4 // Assuming 4 bytes per data point
    };
  }

  dispose(): void {
    this.canvas = null;
    this.data = new Uint8Array();
    this.isInitialized = false;
  }
}

// Mock WASM module
export const mockWasmModule: MockWasmModule = {
  ChartRenderer: MockChartRenderer,
  initialize_logging: () => {
    console.log('Mock WASM logging initialized');
  },
  get_version: () => '0.1.0-mock'
};

// Export default for ES module compatibility
export default mockWasmModule;

// Mock the WASM initialization function
export const mockWasmInit = async (): Promise<MockWasmModule> => {
  // Simulate WASM loading delay
  await new Promise(resolve => setTimeout(resolve, 200));
  return mockWasmModule;
};

// Additional test utilities
export const testUtils = {
  createMockChart: (): MockChartRenderer => new MockChartRenderer(),
  
  generateMockData: (points: number = 1000): Uint8Array => {
    const data = new Uint8Array(points * 4 * 4); // 4 columns, 4 bytes each
    const view = new DataView(data.buffer);
    
    let offset = 0;
    const baseTime = Date.now() - 86400000; // 24 hours ago
    let basePrice = 45000;
    
    for (let i = 0; i < points; i++) {
      // time (4 bytes)
      view.setUint32(offset, baseTime + i * 60000, true); // 1 minute intervals
      offset += 4;
      
      // price (4 bytes)
      basePrice += (Math.random() - 0.5) * 100;
      view.setFloat32(offset, basePrice, true);
      offset += 4;
      
      // volume (4 bytes)
      view.setFloat32(offset, Math.random() * 10000, true);
      offset += 4;
      
      // side (4 bytes)
      view.setUint32(offset, Math.random() > 0.5 ? 1 : 0, true);
      offset += 4;
    }
    
    return data;
  },
  
  waitForNextTick: (): Promise<void> => new Promise(resolve => setTimeout(resolve, 0)),
  
  simulateCanvasResize: (canvas: HTMLCanvasElement, width: number, height: number): void => {
    Object.defineProperty(canvas, 'width', { value: width, configurable: true });
    Object.defineProperty(canvas, 'height', { value: height, configurable: true });
    Object.defineProperty(canvas, 'clientWidth', { value: width, configurable: true });
    Object.defineProperty(canvas, 'clientHeight', { value: height, configurable: true });
    
    // Trigger resize event
    const resizeEvent = new Event('resize');
    canvas.dispatchEvent(resizeEvent);
  }
};