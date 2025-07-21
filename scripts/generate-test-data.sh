#!/bin/bash
# Generate test datasets for performance benchmarking

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DATASETS_DIR="$PROJECT_ROOT/benchmarks/datasets"

echo "Generating test datasets in $DATASETS_DIR..."

# Create datasets directory if it doesn't exist
mkdir -p "$DATASETS_DIR"

# Generate different sized datasets
generate_dataset() {
    local name=$1
    local rows=$2
    local output="$DATASETS_DIR/${name}.bin"
    
    echo "Generating $name dataset ($rows rows)..."
    
    # Header (100 bytes)
    printf '\x00%.0s' {1..100} > "$output"
    
    # Data rows (8 bytes each: 4 byte timestamp + 4 byte float)
    for ((i=0; i<$rows; i++)); do
        # Timestamp (little-endian u32)
        printf '\x%02x\x%02x\x%02x\x%02x' \
            $((i & 0xFF)) \
            $(((i >> 8) & 0xFF)) \
            $(((i >> 16) & 0xFF)) \
            $(((i >> 24) & 0xFF)) >> "$output"
        
        # Price (little-endian f32, simplified)
        printf '\x00\x00\x80\x3F' >> "$output"  # 1.0 in IEEE 754
        
        # Progress indicator
        if [ $((i % 100000)) -eq 0 ]; then
            echo -ne "\r  Progress: $i / $rows"
        fi
    done
    echo -e "\r  Done: $rows rows, $(stat -f%z "$output" 2>/dev/null || stat -c%s "$output") bytes"
}

# Generate test datasets
generate_dataset "small_1k" 1000
generate_dataset "medium_100k" 100000
generate_dataset "large_1m" 1000000

# Note: Larger datasets (10M, 100M, 1B) would be generated on demand
# due to file size constraints

echo "Test datasets generated successfully!"
echo ""
echo "To run benchmarks:"
echo "  cd $PROJECT_ROOT"
echo "  cargo bench --package gpu-charts-data"
echo "  cargo bench --package gpu-charts-renderer"