// Fibonacci benchmark for Go
// Compare against Kujo implementation

package main

import (
	"fmt"
	"time"
)

func fibonacci(n int) int {
	if n <= 1 {
		return n
	}
	return fibonacci(n-1) + fibonacci(n-2)
}

func benchmark(iterations int) {
	start := time.Now()
	
	for i := 0; i < iterations; i++ {
		result := fibonacci(30)
		if i == 0 {
			fmt.Printf("Fib(30) = %d\n", result)
		}
	}
	
	elapsed := time.Since(start)
	avg := elapsed.Seconds() / float64(iterations)
	
	fmt.Printf("\nIterations: %d\n", iterations)
	fmt.Printf("Total time: %.3fs\n", elapsed.Seconds())
	fmt.Printf("Average time: %.3fs\n", avg)
	fmt.Printf("Ops/sec: %.2f\n", 1.0/avg)
}

func main() {
	fmt.Println("Go Fibonacci Benchmark")
	fmt.Println("======================")
	benchmark(10)
}
