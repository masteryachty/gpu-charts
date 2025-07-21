#!/bin/bash
# Build script for multi-crate WASM architecture

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WEB_PKG_DIR="$PROJECT_ROOT/web/pkg"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building GPU Charts WASM modules...${NC}"

# Function to build a WASM crate
build_wasm_crate() {
    local crate_path=$1
    local crate_name=$2
    local output_name=$3
    
    echo -e "${YELLOW}Building $crate_name...${NC}"
    
    cd "$PROJECT_ROOT/$crate_path"
    
    if [ "$BUILD_MODE" = "release" ]; then
        wasm-pack build --target web --out-dir "$WEB_PKG_DIR" \
            --out-name "$output_name" --release
    else
        wasm-pack build --target web --out-dir "$WEB_PKG_DIR" \
            --out-name "$output_name" --dev
    fi
    
    echo -e "${GREEN}✓ $crate_name built successfully${NC}"
}

# Parse build mode
BUILD_MODE="${1:-dev}"
echo "Build mode: $BUILD_MODE"

# Create output directory
mkdir -p "$WEB_PKG_DIR"

# Build shared types (TypeScript generation)
echo -e "${YELLOW}Generating TypeScript types...${NC}"
cd "$PROJECT_ROOT/crates/shared-types"
cargo build --features typescript
echo -e "${GREEN}✓ Types generated${NC}"

# Build WASM bridge (includes data-manager and renderer)
build_wasm_crate "crates/wasm-bridge" "WASM Bridge" "gpu_charts"

# Generate combined package.json
echo -e "${YELLOW}Creating package.json...${NC}"
cat > "$WEB_PKG_DIR/package.json" << EOF
{
  "name": "gpu-charts-wasm",
  "version": "0.1.0",
  "files": [
    "gpu_charts_bg.wasm",
    "gpu_charts.js",
    "gpu_charts.d.ts"
  ],
  "module": "gpu_charts.js",
  "types": "gpu_charts.d.ts",
  "sideEffects": false
}
EOF

# Report bundle sizes
echo -e "\n${GREEN}Build complete! Bundle sizes:${NC}"
if [ -f "$WEB_PKG_DIR/gpu_charts_bg.wasm" ]; then
    WASM_SIZE=$(stat -f%z "$WEB_PKG_DIR/gpu_charts_bg.wasm" 2>/dev/null || stat -c%s "$WEB_PKG_DIR/gpu_charts_bg.wasm")
    echo "  WASM: $(echo "scale=2; $WASM_SIZE / 1024 / 1024" | bc) MB"
    
    # Check gzipped size
    if command -v gzip &> /dev/null; then
        GZIP_SIZE=$(gzip -c "$WEB_PKG_DIR/gpu_charts_bg.wasm" | wc -c)
        echo "  WASM (gzipped): $(echo "scale=2; $GZIP_SIZE / 1024" | bc) KB"
    fi
fi

# Performance check
if [ "$BUILD_MODE" = "release" ]; then
    echo -e "\n${YELLOW}Checking performance targets...${NC}"
    
    # Check WASM size is under 500KB gzipped
    if [ -n "$GZIP_SIZE" ] && [ "$GZIP_SIZE" -lt 512000 ]; then
        echo -e "${GREEN}✓ WASM size under 500KB gzipped${NC}"
    else
        echo -e "${RED}✗ WASM size exceeds 500KB gzipped target${NC}"
    fi
fi

echo -e "\n${GREEN}Build successful!${NC}"
echo "Output directory: $WEB_PKG_DIR"