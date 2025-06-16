// TypeScript declarations for WASM modules

declare module '*.wasm' {
  const wasmModule: WebAssembly.Module;
  export default wasmModule;
}

declare module '@pkg/tutorial1_window' {
  // Main initialization function
  export default function init(input?: any): Promise<any>;
  
  // Export the main run function
  export function run(): void;
  
  // Manual run function for React integration
  export function manual_run(): void;
  
  // Advanced chart class for React integration with store bridge
  export class Chart {
    constructor();
    init(canvas_id: string, width: number, height: number): Promise<void>;
    
    // Core bridge method - the main integration point
    update_chart_state(store_state_json: string): Promise<string>;
    
    // Smart change detection methods
    configure_change_detection(config_json: string): Promise<string>;
    get_change_detection_config(): Promise<string>;
    detect_state_changes(store_state_json: string): Promise<string>;
    
    // Utility methods
    is_initialized(): boolean;
    get_current_store_state(): Promise<string>;
    force_update_chart_state(store_state_json: string): Promise<string>;
    
    // Rendering and interaction
    render(): Promise<void>;
    resize(width: number, height: number): void;
    handle_mouse_wheel(delta_y: number, x: number, y: number): void;
    handle_mouse_move(x: number, y: number): void;
    handle_mouse_click(x: number, y: number, pressed: boolean): void;
    request_redraw(): void;
    set_data_range(start: number, end: number): void;
  }

  // Main chart class - uses WasmCanvas for full-featured rendering
  export class SimpleChart {
    constructor();
    init(canvas_id: string): void;
    is_initialized(): boolean;
    
    // Optional extended functionality (may not be available in current build)
    update_state?(symbol: string, timeframe: string, connected: boolean): void;
    render?(): Promise<void>;
    
    // Change detection (may not be available in current build)  
    configure_change_detection?(config: any): Promise<boolean>;
    get_change_detection_config?(): Promise<any>;
    detect_changes?(storeState: any): Promise<any>;
    get_current_state?(): Promise<any>;
    
    // Mouse interactions (may not be available in current build)
    handle_mouse_wheel?(delta: number, x: number, y: number): void;
    handle_mouse_move?(x: number, y: number): void;
    handle_mouse_click?(x: number, y: number, pressed: boolean): void;
  }
}

declare module '@pkg/tutorial1_window.js' {
  // Main initialization function
  export default function init(input?: any): Promise<any>;
  
  // Export the main run function
  export function run(): void;
  
  // Manual run function for React integration
  export function manual_run(): void;
  
  // Advanced chart class for React integration with store bridge
  export class Chart {
    constructor();
    init(canvas_id: string, width: number, height: number): Promise<void>;
    
    // Core bridge method - the main integration point
    update_chart_state(store_state_json: string): Promise<string>;
    
    // Utility methods
    is_initialized(): boolean;
    get_current_store_state(): Promise<string>;
    force_update_chart_state(store_state_json: string): Promise<string>;
    
    // Rendering and interaction
    render(): Promise<void>;
    resize(width: number, height: number): void;
    handle_mouse_wheel(delta_y: number, x: number, y: number): void;
    handle_mouse_move(x: number, y: number): void;
    handle_mouse_click(x: number, y: number, pressed: boolean): void;
    request_redraw(): void;
    set_data_range(start: number, end: number): void;
  }

  // Main chart class - uses WasmCanvas for full-featured rendering
  export class SimpleChart {
    constructor();
    init(canvas_id: string): void;
    is_initialized(): boolean;
    
    // Optional extended functionality (may not be available in current build)
    update_state?(symbol: string, timeframe: string, connected: boolean): void;
    render?(): Promise<void>;
    
    // Change detection (may not be available in current build)  
    configure_change_detection?(config: any): Promise<boolean>;
    get_change_detection_config?(): Promise<any>;
    detect_changes?(storeState: any): Promise<any>;
    get_current_state?(): Promise<any>;
    
    // Mouse interactions (may not be available in current build)
    handle_mouse_wheel?(delta: number, x: number, y: number): void;
    handle_mouse_move?(x: number, y: number): void;
    handle_mouse_click?(x: number, y: number, pressed: boolean): void;
  }
}

// Global types for WASM memory and utilities
declare global {
  interface Window {
    wasmModule?: any;
    copyToWasm?: (srcArrayBuffer: ArrayBuffer, ptr: number, len: number) => void;
  }
}

export {};