declare global {
  interface Window {
    wasmPromise?: Promise<typeof import('@pkg/wasm_bridge.js')>;
  }
}

export {};