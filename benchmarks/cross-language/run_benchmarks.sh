#!/bin/bash

# Cross-Language Benchmark Runner
# Runs Kujo, Python, and Go benchmarks and compares results

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║         KUJO vs PYTHON vs GO - PERFORMANCE BENCHMARK          ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

# Check prerequisites
echo "Checking prerequisites..."

if ! command -v python3 &> /dev/null; then
    echo "❌ Python 3 not found. Please install Python 3."
    exit 1
fi

if ! command -v go &> /dev/null; then
    echo "❌ Go not found. Please install Go."
    exit 1
fi

if [ ! -f "../../target/release/kujo" ]; then
    echo "❌ Kujo binary not found. Building..."
    cd ../..
    cargo build --release
    cd "$SCRIPT_DIR"
fi

echo "✓ All prerequisites satisfied"
echo ""

# Create results directory
mkdir -p results
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULTS_FILE="results/benchmark_${TIMESTAMP}.txt"

echo "Results will be saved to: $RESULTS_FILE"
echo ""

# Compile Go benchmark
echo "Compiling Go benchmark..."
go build -o bench_go bench.go
echo "✓ Go benchmark compiled"
echo ""

# Run benchmarks
echo "════════════════════════════════════════════════════════════════"
echo "RUNNING BENCHMARKS (this may take a few minutes)..."
echo "════════════════════════════════════════════════════════════════"
echo ""

{
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║         KUJO vs PYTHON vs GO - PERFORMANCE BENCHMARK          ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo ""
    echo "Generated: $(date)"
    echo "System: $(uname -s) $(uname -m)"
    echo ""
} > "$RESULTS_FILE"

# Run Kujo
echo "🦀 Running Kujo benchmark..."
echo "────────────────────────────────────────────────────────────────" | tee -a "$RESULTS_FILE"
echo "KUJO BENCHMARK" | tee -a "$RESULTS_FILE"
echo "────────────────────────────────────────────────────────────────" | tee -a "$RESULTS_FILE"
../../target/release/kujo run bench.kujo 2>&1 | tee -a "$RESULTS_FILE"
echo "" | tee -a "$RESULTS_FILE"

# Run Python
echo "🐍 Running Python benchmark..."
echo "────────────────────────────────────────────────────────────────" | tee -a "$RESULTS_FILE"
echo "PYTHON BENCHMARK" | tee -a "$RESULTS_FILE"
echo "────────────────────────────────────────────────────────────────" | tee -a "$RESULTS_FILE"
python3 bench.py 2>&1 | tee -a "$RESULTS_FILE"
echo "" | tee -a "$RESULTS_FILE"

# Run Go
echo "🐹 Running Go benchmark..."
echo "────────────────────────────────────────────────────────────────" | tee -a "$RESULTS_FILE"
echo "GO BENCHMARK" | tee -a "$RESULTS_FILE"
echo "────────────────────────────────────────────────────────────────" | tee -a "$RESULTS_FILE"
./bench_go 2>&1 | tee -a "$RESULTS_FILE"
echo "" | tee -a "$RESULTS_FILE"

# Parse results and create comparison
echo "════════════════════════════════════════════════════════════════"
echo "ANALYSIS COMPLETE"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "Full results saved to: $RESULTS_FILE"
echo ""
echo "Quick Summary:"
echo "  - Check results file for detailed timings"
echo "  - Compare 'Time: XXms' lines for each benchmark"
echo "  - Lower is better!"
echo ""
echo "Key Benchmarks to Compare:"
echo "  1. Fibonacci Recursive - Function call overhead"
echo "  2. Fibonacci Iterative - Loop performance"
echo "  3. Array Sum - Iteration speed"
echo "  4. Hash Map Operations - Dictionary performance"
echo "  5. Nested Loops - Optimization quality"
echo ""

# Cleanup
rm -f bench_go

echo "✨ Benchmark complete! Check the results file for detailed comparison."
