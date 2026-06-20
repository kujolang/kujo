#!/bin/bash
# Cross-language Performance Comparison Script
#
# Runs equivalent benchmarks across Kujo, Go, Python, and Node.js
# to compare raw performance characteristics.

set -e

echo "=================================="
echo "Cross-Language Performance Comparison"
echo "=================================="
echo ""

# Check if required languages are installed
check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo "Warning: $1 not found, skipping $2 benchmarks"
        return 1
    fi
    return 0
}

# Fibonacci Benchmark
echo "1. Fibonacci (Recursive) Benchmark"
echo "----------------------------------"

if check_command "cargo"; then
    echo "Running Kujo..."
    cargo run --release -- run examples/benchmarks/fibonacci.kujo
    echo ""
fi

if check_command "go"; then
    echo "Running Go..."
    cd examples/benchmarks
    go run fibonacci_go.go
    cd ../..
    echo ""
fi

if check_command "python3"; then
    echo "Running Python..."
    python3 examples/benchmarks/fibonacci_python.py
    echo ""
fi

if check_command "node"; then
    echo "Running Node.js..."
    node examples/benchmarks/fibonacci_node.js
    echo ""
fi

# Array Operations Benchmark
echo ""
echo "2. Array Operations Benchmark"
echo "------------------------------"

if check_command "cargo"; then
    echo "Running Kujo..."
    cargo run --release -- run examples/benchmarks/array_ops_comparison.kujo
    echo ""
fi

if check_command "python3"; then
    echo "Running Python..."
    python3 examples/benchmarks/array_ops_python.py
    echo ""
fi

if check_command "node"; then
    echo "Running Node.js..."
    node examples/benchmarks/array_ops_node.js
    echo ""
fi

# Summary
echo ""
echo "=================================="
echo "Comparison Complete!"
echo "=================================="
echo ""
echo "Notes:"
echo "- This helper is for local exploration, not v1.0 publication claims."
echo "- Record tool versions, commands, raw logs, and correctness checks before citing results."
echo "- See docs/BENCHMARK_PUBLICATION_POLICY.md for publication requirements."
echo ""
echo "For detailed profiling, run:"
echo "  cargo run --release -- profile <file> --flamegraph profile.txt"
echo ""
