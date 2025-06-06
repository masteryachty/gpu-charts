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
  
  // Simple chart class for React integration
  export class SimpleChart {
    constructor();
    init(canvas_id: string): void;
    is_initialized(): boolean;
  }
}

declare module '@pkg/tutorial1_window.js' {
  // Main initialization function
  export default function init(input?: any): Promise<any>;
  
  // Export the main run function
  export function run(): void;
  
  // Manual run function for React integration
  export function manual_run(): void;
  
  // Simple chart class for React integration
  export class SimpleChart {
    constructor();
    init(canvas_id: string): void;
    is_initialized(): boolean;
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