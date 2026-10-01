package main

import "fmt"

func countPrimes(limit int) int {
	count := 0
	for n := 2; n < limit; n++ {
		isPrime := true
		for divisor := 2; divisor*divisor <= n; divisor++ {
			if n%divisor == 0 {
				isPrime = false
				break
			}
		}
		if isPrime {
			count++
		}
	}
	return count
}

func main() {
	fmt.Printf("prime_count=%d\n", countPrimes(50000))
}
