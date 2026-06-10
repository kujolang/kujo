#!/usr/bin/env python3
"""
Python Cross-Language Benchmark Suite
Equivalent implementations to bench.kujo
"""

import time

# ============================================================================
# 1. FIBONACCI (Recursive) - Tests function call overhead
# ============================================================================
def fib_recursive(n):
    if n <= 1:
        return n
    return fib_recursive(n - 1) + fib_recursive(n - 2)

# ============================================================================
# 2. FIBONACCI (Iterative) - Tests loop performance
# ============================================================================
def fib_iterative(n):
    if n <= 1:
        return n
    
    a = 0
    b = 1
    
    for i in range(2, n + 1):
        a, b = b, a + b
    
    return b

# ============================================================================
# 3. ARRAY SUM - Tests array iteration
# ============================================================================
def array_sum(arr):
    total = 0
    for item in arr:
        total += item
    return total

# ============================================================================
# 4. HASH MAP OPERATIONS - Tests dictionary performance
# ============================================================================
def hash_map_ops(n):
    map_dict = {}
    
    # Insert
    for i in range(n):
        map_dict[i] = i * 2
    
    # Read
    total = 0
    for i in range(n):
        total += map_dict[i]
    
    return total

# ============================================================================
# 5. STRING CONCATENATION - Tests string operations
# ============================================================================
def string_concat(n):
    result = ""
    for i in range(n):
        result += "x"
    return len(result)

# ============================================================================
# 6. NESTED LOOPS - Tests loop optimization
# ============================================================================
def nested_loops(n):
    total = 0
    for i in range(n):
        for j in range(n):
            total += 1
    return total

# ============================================================================
# 7. ARRAY BUILDING - Tests array construction
# ============================================================================
def build_array(n):
    arr = []
    for i in range(n):
        arr.append(i)
    return len(arr)

# ============================================================================
# 8. OBJECT CREATION - Tests allocation performance
# ============================================================================
def object_creation(n):
    objects = []
    for i in range(n):
        obj = {"id": i, "value": i * 2, "name": "object"}
        objects.append(obj)
    return len(objects)

# ============================================================================
# BENCHMARK RUNNER
# ============================================================================

def run_benchmarks():
    print("=== PYTHON BENCHMARK SUITE ===")
    print()
    
    # Warm up
    fib_recursive(10)
    fib_iterative(100)
    
    # 1. Fibonacci Recursive
    print("1. Fibonacci Recursive (n=30)...")
    start = time.time()
    result = fib_recursive(30)
    elapsed = (time.time() - start) * 1000
    print(f"   Result: {result}")
    print(f"   Time: {elapsed:.2f}ms")
    print()
    
    # 2. Fibonacci Iterative
    print("2. Fibonacci Iterative (n=100000)...")
    start = time.time()
    result = fib_iterative(100000)
    elapsed = (time.time() - start) * 1000
    print(f"   Result: {result}")
    print(f"   Time: {elapsed:.2f}ms")
    print()
    
    # 3. Array Sum
    print("3. Array Sum (1M elements)...")
    arr = list(range(1000000))
    start = time.time()
    result = array_sum(arr)
    elapsed = (time.time() - start) * 1000
    print(f"   Result: {result}")
    print(f"   Time: {elapsed:.2f}ms")
    print()
    
    # 4. Hash Map Operations
    print("4. Hash Map Operations (100k items)...")
    start = time.time()
    result = hash_map_ops(100000)
    elapsed = (time.time() - start) * 1000
    print(f"   Result: {result}")
    print(f"   Time: {elapsed:.2f}ms")
    print()
    
    # 5. String Concatenation
    print("5. String Concatenation (10k chars)...")
    start = time.time()
    result = string_concat(10000)
    elapsed = (time.time() - start) * 1000
    print(f"   Result: {result} chars")
    print(f"   Time: {elapsed:.2f}ms")
    print()
    
    # 6. Nested Loops
    print("6. Nested Loops (1000x1000)...")
    start = time.time()
    result = nested_loops(1000)
    elapsed = (time.time() - start) * 1000
    print(f"   Result: {result}")
    print(f"   Time: {elapsed:.2f}ms")
    print()
    
    # 7. Array Building
    print("7. Array Building (100k elements)...")
    start = time.time()
    result = build_array(100000)
    elapsed = (time.time() - start) * 1000
    print(f"   Result: {result} elements")
    print(f"   Time: {elapsed:.2f}ms")
    print()
    
    # 8. Object Creation
    print("8. Object Creation (100k objects)...")
    start = time.time()
    result = object_creation(100000)
    elapsed = (time.time() - start) * 1000
    print(f"   Result: {result} objects")
    print(f"   Time: {elapsed:.2f}ms")
    print()
    
    print("=== BENCHMARK COMPLETE ===")

if __name__ == "__main__":
    run_benchmarks()
