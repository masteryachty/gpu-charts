# GPU Charts

WebAssembly-based real-time data visualization application built in Rust that renders interactive line graphs using WebGPU for high-performance GPU-accelerated rendering.

## Prerequisites

- Node.js (v18+)
- Rust toolchain
- wasm-pack

## Quick Start

1. **Install dependencies**:
   ```bash
   npm install
   ```

2. **Build WASM module** (first time only):
   ```bash
   npm run dev:wasm
   ```

3. **Run development server**:
   ```bash
   npm run dev:full
   ```

## Development Commands

- `npm run dev:full` - Run WASM watch + React dev server
- `npm run dev:suite` - Run full suite with data server
- `npm run dev:wasm` - Build WASM module once
- `npm run build` - Production build

## Accessing the Application

Once running, access the application at:
- Local: http://localhost:3000/
- With parameters: http://localhost:3000/app?topic=BTC-usd&start=1745322750&end=1745691150

## Project Structure

- `/crates` - Modular Rust crates for the charting system
  - `wasm-bridge` - Central orchestration and JavaScript bridge
  - `data-manager` - Data operations and GPU buffer management
  - `renderer` - Pure GPU rendering engine
  - `config-system` - Configuration management
  - `shared-types` - Common types and interfaces
- `/web` - React frontend application
- `/server` - High-performance data server
- `/coinbase-logger` - Real-time market data logger