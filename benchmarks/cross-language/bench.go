package main

import (
	"fmt"
	"time"
)

// ============================================================================
// 1. FIBONACCI (Recursive) - Tests function call overhead
// ============================================================================
func fibRecursive(n int) int {
	if n <= 1 {
		return n
	}
	return fibRecursive(n-1) + fibRecursive(n-2)
}

// ============================================================================
// 2. FIBONACCI (Iterative) - Tests loop performance
// ============================================================================
func fibIterative(n int) int {
	if n <= 1 {
		return n
	}

	a := 0
	b := 1

	for i := 2; i <= n; i++ {
		a, b = b, a+b
	}

	return b
}

// ============================================================================
// 3. ARRAY SUM - Tests array iteration
// ============================================================================
func arraySum(arr []int) int {
	sum := 0
	for _, item := range arr {
		sum += item
	}
	return sum
}

// ============================================================================
// 4. HASH MAP OPERATIONS - Tests dictionary performance
// ============================================================================
func hashMapOps(n int) int {
	m := make(map[int]int)

	// Insert
	for i := 0; i < n; i++ {
		m[i] = i * 2
	}

	// Read
	sum := 0
	for i := 0; i < n; i++ {
		sum += m[i]
	}

	return sum
}

// ============================================================================
// 5. STRING CONCATENATION - Tests string operations
// ============================================================================
func stringConcat(n int) int {
	result := ""
	for i := 0; i < n; i++ {
		result += "x"
	}
	return len(result)
}

// ============================================================================
// 6. NESTED LOOPS - Tests loop optimization
// ============================================================================
func nestedLoops(n int) int {
	sum := 0
	for i := 0; i < n; i++ {
		for j := 0; j < n; j++ {
			sum++
		}
	}
	return sum
}

// ============================================================================
// 7. ARRAY BUILDING - Tests array construction
// ============================================================================
func buildArray(n int) int {
	arr := make([]int, 0, n)
	for i := 0; i < n; i++ {
		arr = append(arr, i)
	}
	return len(arr)
}

// ============================================================================
// 8. OBJECT CREATION - Tests allocation performance
// ============================================================================
type Object struct {
	ID    int
	Value int
	Name  string
}

func objectCreation(n int) int {
	objects := make([]Object, 0, n)
	for i := 0; i < n; i++ {
		obj := Object{
			ID:    i,
			Value: i * 2,
			Name:  "object",
		}
		objects = append(objects, obj)
	}
	return len(objects)
}

// ============================================================================
// BENCHMARK RUNNER
// ============================================================================

func main() {
	fmt.Println("=== GO BENCHMARK SUITE ===")
	fmt.Println()

	// Warm up
	fibRecursive(10)
	fibIterative(100)

	// 1. Fibonacci Recursive
	fmt.Println("1. Fibonacci Recursive (n=30)...")
	start := time.Now()
	result := fibRecursive(30)
	elapsed := time.Since(start).Milliseconds()
	fmt.Printf("   Result: %d\n", result)
	fmt.Printf("   Time: %dms\n", elapsed)
	fmt.Println()

	// 2. Fibonacci Iterative
	fmt.Println("2. Fibonacci Iterative (n=100000)...")
	start = time.Now()
	result = fibIterative(100000)
	elapsed = time.Since(start).Milliseconds()
	fmt.Printf("   Result: %d\n", result)
	fmt.Printf("   Time: %dms\n", elapsed)
	fmt.Println()

	// 3. Array Sum
	fmt.Println("3. Array Sum (1M elements)...")
	arr := make([]int, 1000000)
	for i := range arr {
		arr[i] = i
	}
	start = time.Now()
	sum := arraySum(arr)
	elapsed = time.Since(start).Milliseconds()
	fmt.Printf("   Result: %d\n", sum)
	fmt.Printf("   Time: %dms\n", elapsed)
	fmt.Println()

	// 4. Hash Map Operations
	fmt.Println("4. Hash Map Operations (100k items)...")
	start = time.Now()
	result = hashMapOps(100000)
	elapsed = time.Since(start).Milliseconds()
	fmt.Printf("   Result: %d\n", result)
	fmt.Printf("   Time: %dms\n", elapsed)
	fmt.Println()

	// 5. String Concatenation
	fmt.Println("5. String Concatenation (10k chars)...")
	start = time.Now()
	result = stringConcat(10000)
	elapsed = time.Since(start).Milliseconds()
	fmt.Printf("   Result: %d chars\n", result)
	fmt.Printf("   Time: %dms\n", elapsed)
	fmt.Println()

	// 6. Nested Loops
	fmt.Println("6. Nested Loops (1000x1000)...")
	start = time.Now()
	result = nestedLoops(1000)
	elapsed = time.Since(start).Milliseconds()
	fmt.Printf("   Result: %d\n", result)
	fmt.Printf("   Time: %dms\n", elapsed)
	fmt.Println()

	// 7. Array Building
	fmt.Println("7. Array Building (100k elements)...")
	start = time.Now()
	result = buildArray(100000)
	elapsed = time.Since(start).Milliseconds()
	fmt.Printf("   Result: %d elements\n", result)
	fmt.Printf("   Time: %dms\n", elapsed)
	fmt.Println()

	// 8. Object Creation
	fmt.Println("8. Object Creation (100k objects)...")
	start = time.Now()
	result = objectCreation(100000)
	elapsed = time.Since(start).Milliseconds()
	fmt.Printf("   Result: %d objects\n", result)
	fmt.Printf("   Time: %dms\n", elapsed)
	fmt.Println()

	fmt.Println("=== BENCHMARK COMPLETE ===")
}
