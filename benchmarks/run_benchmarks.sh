#!/bin/bash

# GPU Charts Comprehensive Benchmark Suite
# Run all benchmarks and generate reports

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
RESULTS_DIR="benchmark_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_DIR="${RESULTS_DIR}/${TIMESTAMP}"

# Create results directory
mkdir -p "${REPORT_DIR}"

echo -e "${GREEN}GPU Charts Benchmark Suite${NC}"
echo "=============================="
echo "Results will be saved to: ${REPORT_DIR}"
echo ""

# Function to run a benchmark
run_benchmark() {
    local name=$1
    local bench_name=$2
    
    echo -e "${YELLOW}Running ${name}...${NC}"
    
    # Run the benchmark
    if cargo bench --bench "${bench_name}" -- --save-baseline "${TIMESTAMP}_${bench_name}" 2>&1 | tee "${REPORT_DIR}/${bench_name}.log"; then
        echo -e "${GREEN}✓ ${name} completed${NC}"
        
        # Copy HTML report if it exists
        if [ -d "target/criterion/${bench_name}" ]; then
            cp -r "target/criterion/${bench_name}" "${REPORT_DIR}/"
        fi
    else
        echo -e "${RED}✗ ${name} failed${NC}"
        return 1
    fi
    
    echo ""
}

# System information
echo "System Information" > "${REPORT_DIR}/system_info.txt"
echo "==================" >> "${REPORT_DIR}/system_info.txt"
echo "" >> "${REPORT_DIR}/system_info.txt"

# OS info
echo "OS: $(uname -a)" >> "${REPORT_DIR}/system_info.txt"

# CPU info
if [ -f /proc/cpuinfo ]; then
    echo "CPU: $(grep 'model name' /proc/cpuinfo | head -1 | cut -d':' -f2 | xargs)" >> "${REPORT_DIR}/system_info.txt"
    echo "Cores: $(nproc)" >> "${REPORT_DIR}/system_info.txt"
fi

# Memory info
if [ -f /proc/meminfo ]; then
    echo "Memory: $(grep MemTotal /proc/meminfo | awk '{print $2/1024/1024 " GB"}')" >> "${REPORT_DIR}/system_info.txt"
fi

# GPU info (if available)
if command -v nvidia-smi &> /dev/null; then
    echo "" >> "${REPORT_DIR}/system_info.txt"
    echo "GPU Information:" >> "${REPORT_DIR}/system_info.txt"
    nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader >> "${REPORT_DIR}/system_info.txt"
fi

echo ""

# Run all benchmarks
FAILED=0

run_benchmark "Data Loading Benchmarks" "data_loading" || FAILED=$((FAILED + 1))
run_benchmark "Rendering Benchmarks" "rendering" || FAILED=$((FAILED + 1))
run_benchmark "Memory Usage Benchmarks" "memory_usage" || FAILED=$((FAILED + 1))
run_benchmark "End-to-End Benchmarks" "end_to_end" || FAILED=$((FAILED + 1))
run_benchmark "Stress Tests" "stress_test" || FAILED=$((FAILED + 1))
run_benchmark "Phase 2 Real Benchmarks" "phase2_real" || FAILED=$((FAILED + 1))
run_benchmark "Phase 2 Comparison" "phase2_comparison" || FAILED=$((FAILED + 1))

# Generate summary report
echo -e "${YELLOW}Generating summary report...${NC}"

cat > "${REPORT_DIR}/summary.md" << EOF
# GPU Charts Benchmark Report

**Date**: $(date)
**System**: $(uname -n)

## Summary

Total benchmarks run: 7
Failed benchmarks: ${FAILED}

## Results

### Data Loading Performance
- Binary parsing throughput
- GPU buffer preparation
- Cache operations
- Data validation

### Rendering Performance
- Vertex generation speed
- Culling efficiency
- LOD selection
- Draw call optimization
- Overlay composition

### Memory Usage
- Buffer pool efficiency
- Memory fragmentation
- GPU memory transfer
- Zero-copy operations

### End-to-End Scenarios
- Small dataset (1K points)
- Medium dataset (100K points)
- Large dataset (10M points)
- Interactive scenarios (zoom/pan)

### Stress Tests
- Billion point simulation
- Memory limit testing
- 50 concurrent charts
- Sustained 60 FPS load

## Performance vs Targets

Target metrics from PERFORMANCE_GUIDE.md:
- Frame time: <16ms (60 FPS)
- GPU time: <14ms
- CPU time: <5ms
- Draw calls: <100

See individual benchmark reports for detailed results.

## Recommendations

Based on the benchmark results, consider:
1. Optimizing any operations that exceed target times
2. Investigating memory usage patterns
3. Improving cache hit rates
4. Reducing draw call counts where possible

EOF

# Compare with baseline if exists
if [ -d "target/criterion" ]; then
    echo -e "${YELLOW}Comparing with baseline...${NC}"
    
    # List available baselines
    for bench in data_loading rendering memory_usage end_to_end stress_test; do
        if cargo bench --bench "${bench}" -- --baseline "${TIMESTAMP}_${bench}" --compare 2>/dev/null | grep -E "(Performance|Change)"; then
            echo "Comparison for ${bench} saved to report"
        fi
    done
fi

# Create index.html for easy navigation
cat > "${REPORT_DIR}/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>GPU Charts Benchmark Report - ${TIMESTAMP}</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        h1 { color: #333; }
        .benchmark { margin: 20px 0; padding: 10px; border: 1px solid #ddd; }
        .success { background-color: #e7f5e7; }
        .failure { background-color: #f5e7e7; }
        a { color: #0066cc; text-decoration: none; }
        a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <h1>GPU Charts Benchmark Report</h1>
    <p><strong>Generated:</strong> $(date)</p>
    
    <h2>Benchmark Results</h2>
    <div class="benchmark">
        <h3>📊 Data Loading</h3>
        <a href="data_loading/report/index.html">View Report</a> | 
        <a href="data_loading.log">View Log</a>
    </div>
    
    <div class="benchmark">
        <h3>🎨 Rendering</h3>
        <a href="rendering/report/index.html">View Report</a> | 
        <a href="rendering.log">View Log</a>
    </div>
    
    <div class="benchmark">
        <h3>💾 Memory Usage</h3>
        <a href="memory_usage/report/index.html">View Report</a> | 
        <a href="memory_usage.log">View Log</a>
    </div>
    
    <div class="benchmark">
        <h3>🔄 End-to-End</h3>
        <a href="end_to_end/report/index.html">View Report</a> | 
        <a href="end_to_end.log">View Log</a>
    </div>
    
    <div class="benchmark">
        <h3>🔥 Stress Tests</h3>
        <a href="stress_test/report/index.html">View Report</a> | 
        <a href="stress_test.log">View Log</a>
    </div>
    
    <h2>Additional Information</h2>
    <ul>
        <li><a href="system_info.txt">System Information</a></li>
        <li><a href="summary.md">Summary Report</a></li>
    </ul>
</body>
</html>
EOF

echo ""
echo -e "${GREEN}Benchmark suite completed!${NC}"
echo "Results saved to: ${REPORT_DIR}"
echo "Open ${REPORT_DIR}/index.html to view the report"

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Warning: ${FAILED} benchmark(s) failed${NC}"
    exit 1
fi