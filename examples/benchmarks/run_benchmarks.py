# Benchmark Runner Script
# 
# This script runs all benchmarks in both interpreter and VM modes
# and compares the performance.

import subprocess
import time
import statistics
from pathlib import Path

benchmarks = [
    "fibonacci.kujo",
    "primes.kujo",
    "sorting.kujo",
    "strings.kujo",
    "dict_ops.kujo",
    "nested_loops.kujo",
    "higher_order.kujo",
]

def run_benchmark(file, vm_mode=False):
    """Run a single benchmark and extract timing"""
    cmd = ["./target/debug/kujo", "run"]
    if vm_mode:
        cmd.append("--vm")
    cmd.append(f"examples/benchmarks/{file}")
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    output = result.stdout
    
    # Extract time from output (assumes "Time taken: X ms" format)
    for line in output.split('\n'):
        if "Time taken:" in line:
            time_str = line.split("Time taken:")[1].split("ms")[0].strip()
            return float(time_str)
    return None

def main():
    print("=" * 80)
    print("Kujo VM Performance Benchmark Suite")
    print("=" * 80)
    print()
    
    results = []
    
    for benchmark in benchmarks:
        print(f"Running {benchmark}...")
        
        # Run in interpreter mode (3 times, take median)
        interp_times = []
        for _ in range(3):
            t = run_benchmark(benchmark, vm_mode=False)
            if t:
                interp_times.append(t)
        
        # Run in VM mode (3 times, take median)
        vm_times = []
        for _ in range(3):
            t = run_benchmark(benchmark, vm_mode=True)
            if t:
                vm_times.append(t)
        
        if interp_times and vm_times:
            interp_median = statistics.median(interp_times)
            vm_median = statistics.median(vm_times)
            speedup = interp_median / vm_median if vm_median > 0 else 0
            
            results.append({
                "name": benchmark,
                "interp": interp_median,
                "vm": vm_median,
                "speedup": speedup
            })
            
            print(f"  Interpreter: {interp_median:.2f} ms")
            print(f"  VM:          {vm_median:.2f} ms")
            print(f"  Speedup:     {speedup:.2f}x")
            print()
    
    # Summary
    print("=" * 80)
    print("Summary")
    print("=" * 80)
    print()
    print(f"{'Benchmark':<25} {'Interpreter (ms)':<20} {'VM (ms)':<15} {'Speedup':<10}")
    print("-" * 80)
    
    for r in results:
        print(f"{r['name']:<25} {r['interp']:<20.2f} {r['vm']:<15.2f} {r['speedup']:<10.2f}x")
    
    print()
    avg_speedup = statistics.mean([r['speedup'] for r in results])
    print(f"Average speedup: {avg_speedup:.2f}x")
    print()
    
    if avg_speedup >= 10:
        print("✅ SUCCESS: VM achieves 10x+ speedup target!")
    else:
        print(f"⚠️  VM speedup ({avg_speedup:.2f}x) below 10x target")

if __name__ == "__main__":
    main()
