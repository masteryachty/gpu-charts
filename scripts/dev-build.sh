#!/bin/bash

# Development build script for WASM + React hot reloading

echo "🚀 Starting development build pipeline..."

# Function to build WASM
build_wasm() {
    echo "🦀 Building WASM package..."
    cd charting && wasm-pack build --target web --out-dir ../web/pkg --dev --no-opt && cd ..
    
    if [ $? -eq 0 ]; then
        echo "✅ WASM build successful"
        echo "📦 WASM files output to web/pkg/"
        
        # Trigger Vite reload by touching a watched file
        touch web/src/wasm-trigger.ts
        echo "🔄 Triggered React hot reload"
    else
        echo "❌ WASM build failed"
        return 1
    fi
}

# Initial build
build_wasm

# Watch for Rust file changes
echo "👀 Watching for Rust file changes..."
echo "Press Ctrl+C to stop"

# Use inotifywait if available, otherwise use a simple loop
if command -v inotifywait &> /dev/null; then
    # Use inotify for efficient file watching
    while inotifywait -r -e modify,create,delete charting/src/ charting/Cargo.toml crates/*/src/ crates/*/Cargo.toml 2>/dev/null; do
        echo "📝 Rust files changed, rebuilding..."
        build_wasm
        echo "⏰ $(date): Ready for changes..."
    done
else
    # Fallback: simple polling method
    echo "⚠️  inotifywait not found, using polling method"
    echo "💡 Install inotify-tools for better performance: sudo apt install inotify-tools"
    
    last_modified=$(find charting/src/ charting/Cargo.toml crates/*/src/ crates/*/Cargo.toml -type f -exec stat -c %Y {} + 2>/dev/null | sort -n | tail -1)
    
    while true; do
        sleep 2
        current_modified=$(find charting/src/ charting/Cargo.toml crates/*/src/ crates/*/Cargo.toml -type f -exec stat -c %Y {} + 2>/dev/null | sort -n | tail -1)
        
        if [ "$current_modified" != "$last_modified" ]; then
            echo "📝 Rust files changed, rebuilding..."
            build_wasm
            last_modified=$current_modified
            echo "⏰ $(date): Ready for changes..."
        fi
    done
fi