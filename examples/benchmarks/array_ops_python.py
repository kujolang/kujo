#!/usr/bin/env python3
"""
Array Operations Comparison Benchmark for Python
Tests: map, filter, reduce operations on large arrays
"""

import time
from functools import reduce

def benchmark_array_ops(size):
    print(f"Array size: {size}")
    
    # Create array
    arr = list(range(size))
    
    # Map: double each element
    start = time.time()
    doubled = list(map(lambda x: x * 2, arr))
    map_time = (time.time() - start) * 1000
    print(f"Map time: {map_time:.2f}ms")
    
    # Filter: keep only evens
    start = time.time()
    evens = list(filter(lambda x: x % 2 == 0, arr))
    filter_time = (time.time() - start) * 1000
    print(f"Filter time: {filter_time:.2f}ms")
    
    # Reduce: sum all elements
    start = time.time()
    total_sum = reduce(lambda acc, x: acc + x, arr, 0)
    reduce_time = (time.time() - start) * 1000
    print(f"Reduce time: {reduce_time:.2f}ms")
    
    total = map_time + filter_time + reduce_time
    print(f"Total time: {total:.2f}ms")
    print(f"Sum: {total_sum}")

print("Python Array Operations Benchmark")
print("==================================")
benchmark_array_ops(100000)
