#!/usr/bin/env python3
"""
Fibonacci benchmark for Python
Compare against Kujo implementation
"""

import time

def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

def benchmark(iterations):
    start = time.time()
    
    for i in range(iterations):
        result = fibonacci(30)
        if i == 0:
            print(f"Fib(30) = {result}")
    
    elapsed = time.time() - start
    avg = elapsed / iterations
    
    print(f"\nIterations: {iterations}")
    print(f"Total time: {elapsed:.3f}s")
    print(f"Average time: {avg:.3f}s")
    print(f"Ops/sec: {1.0/avg:.2f}")

if __name__ == "__main__":
    print("Python Fibonacci Benchmark")
    print("==========================")
    benchmark(10)
