package main

import "fmt"

func integerMix(iterations int64) int64 {
	accumulator := int64(1)
	for i := int64(1); i <= iterations; i++ {
		accumulator = (accumulator*1664525 + i + 1013904223) % 2147483647
	}
	return accumulator
}

func main() {
	fmt.Printf("integer_mix=%d\n", integerMix(250000))
}
