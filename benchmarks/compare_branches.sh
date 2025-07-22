#!/bin/bash

# GPU Charts Branch Comparison Tool
# Compare performance between different git branches

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
RESULTS_DIR="benchmark_results/comparisons"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Parse arguments
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <branch1> <branch2> [benchmark_name]"
    echo "Example: $0 main feature/phase2"
    echo "Example: $0 main feature/phase2 rendering"
    echo ""
    echo "Available benchmarks:"
    echo "  - data_loading"
    echo "  - rendering"
    echo "  - memory_usage"
    echo "  - end_to_end"
    echo "  - stress_test"
    echo "  - phase2_comparison (if available)"
    exit 1
fi

BRANCH1=$1
BRANCH2=$2
SPECIFIC_BENCH=$3

# Create results directory
mkdir -p "${RESULTS_DIR}"

echo -e "${BLUE}GPU Charts Branch Performance Comparison${NC}"
echo "=========================================="
echo "Comparing: ${BRANCH1} vs ${BRANCH2}"
echo "Timestamp: ${TIMESTAMP}"
echo ""

# Save current branch
CURRENT_BRANCH=$(git branch --show-current)

# Function to run benchmarks on a branch
run_branch_benchmarks() {
    local branch=$1
    local baseline_name=$2
    
    echo -e "${YELLOW}Switching to branch: ${branch}${NC}"
    git checkout "${branch}" --quiet
    
    echo -e "${YELLOW}Building branch: ${branch}${NC}"
    cargo build --release --quiet
    
    # Run specific benchmark or all
    if [ -n "${SPECIFIC_BENCH}" ]; then
        echo -e "${YELLOW}Running ${SPECIFIC_BENCH} benchmark on ${branch}...${NC}"
        cargo bench --bench "${SPECIFIC_BENCH}" -- --save-baseline "${baseline_name}"
    else
        echo -e "${YELLOW}Running all benchmarks on ${branch}...${NC}"
        
        # Run each benchmark with proper error handling
        for bench in data_loading rendering memory_usage end_to_end stress_test; do
            echo -e "  Running ${bench}..."
            if cargo bench --bench "${bench}" -- --save-baseline "${baseline_name}" 2>/dev/null; then
                echo -e "  ${GREEN}✓${NC} ${bench} completed"
            else
                echo -e "  ${RED}✗${NC} ${bench} failed (skipping)"
            fi
        done
    fi
}

# Function to generate comparison report
generate_comparison_report() {
    local report_file="${RESULTS_DIR}/${TIMESTAMP}_${BRANCH1}_vs_${BRANCH2}.md"
    
    cat > "${report_file}" << EOF
# Performance Comparison Report

**Date**: $(date)
**Branches**: \`${BRANCH1}\` vs \`${BRANCH2}\`
**System**: $(uname -n)

## Summary

This report compares the performance between two branches of the GPU Charts project.

### System Information
- **CPU**: $(grep 'model name' /proc/cpuinfo 2>/dev/null | head -1 | cut -d':' -f2 | xargs || echo "Unknown")
- **Cores**: $(nproc)
- **Memory**: $(grep MemTotal /proc/meminfo 2>/dev/null | awk '{print $2/1024/1024 " GB"}' || echo "Unknown")

## Benchmark Results

EOF

    # Add comparison results
    if [ -n "${SPECIFIC_BENCH}" ]; then
        echo "### ${SPECIFIC_BENCH}" >> "${report_file}"
        echo '```' >> "${report_file}"
        cargo bench --bench "${SPECIFIC_BENCH}" -- --load-baseline "${TIMESTAMP}_${BRANCH1}" --baseline "${TIMESTAMP}_${BRANCH2}" 2>&1 | grep -E "(Performance|Change|time:|thrpt:|found)" >> "${report_file}" || true
        echo '```' >> "${report_file}"
    else
        for bench in data_loading rendering memory_usage end_to_end stress_test; do
            echo "### ${bench}" >> "${report_file}"
            echo '```' >> "${report_file}"
            if cargo bench --bench "${bench}" -- --load-baseline "${TIMESTAMP}_${BRANCH1}" --baseline "${TIMESTAMP}_${BRANCH2}" 2>&1 | grep -E "(Performance|Change|time:|thrpt:|found)"; then
                :
            else
                echo "No comparison data available"
            fi >> "${report_file}"
            echo '```' >> "${report_file}"
            echo "" >> "${report_file}"
        done
    fi

    cat >> "${report_file}" << EOF

## Performance Changes

### Improvements
List any benchmarks that showed improvement in ${BRANCH2} compared to ${BRANCH1}.

### Regressions
List any benchmarks that showed regression in ${BRANCH2} compared to ${BRANCH1}.

### Recommendations
Based on the comparison results, consider:
1. Investigating significant regressions
2. Documenting improvements for release notes
3. Running additional targeted benchmarks for changed areas

## Raw Data Location
- Criterion reports: \`target/criterion/\`
- Baseline data: \`target/criterion/*/base/\`
EOF

    echo "${report_file}"
}

# Main execution
echo -e "${BLUE}Step 1: Running benchmarks on ${BRANCH1}${NC}"
run_branch_benchmarks "${BRANCH1}" "${TIMESTAMP}_${BRANCH1}"

echo ""
echo -e "${BLUE}Step 2: Running benchmarks on ${BRANCH2}${NC}"
run_branch_benchmarks "${BRANCH2}" "${TIMESTAMP}_${BRANCH2}"

echo ""
echo -e "${BLUE}Step 3: Generating comparison report${NC}"
REPORT_FILE=$(generate_comparison_report)

# Return to original branch
echo -e "${YELLOW}Returning to branch: ${CURRENT_BRANCH}${NC}"
git checkout "${CURRENT_BRANCH}" --quiet

echo ""
echo -e "${GREEN}Comparison complete!${NC}"
echo "Report saved to: ${REPORT_FILE}"
echo ""
echo "To view detailed Criterion reports, check:"
echo "  target/criterion/*/report/index.html"
echo ""
echo "Quick summary:"
if [ -n "${SPECIFIC_BENCH}" ]; then
    cargo bench --bench "${SPECIFIC_BENCH}" -- --load-baseline "${TIMESTAMP}_${BRANCH1}" --baseline "${TIMESTAMP}_${BRANCH2}" 2>&1 | grep -E "(Performance|Change)" | head -10
else
    echo "Run specific benchmarks for detailed comparisons"
fi