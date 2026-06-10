// Fibonacci benchmark for Node.js
// Compare against Kujo implementation

function fibonacci(n) {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

function benchmark(iterations) {
    const start = Date.now();
    
    for (let i = 0; i < iterations; i++) {
        const result = fibonacci(30);
        if (i === 0) {
            console.log(`Fib(30) = ${result}`);
        }
    }
    
    const elapsed = (Date.now() - start) / 1000;
    const avg = elapsed / iterations;
    
    console.log(`\nIterations: ${iterations}`);
    console.log(`Total time: ${elapsed.toFixed(3)}s`);
    console.log(`Average time: ${avg.toFixed(3)}s`);
    console.log(`Ops/sec: ${(1.0 / avg).toFixed(2)}`);
}

console.log("Node.js Fibonacci Benchmark");
console.log("===========================");
benchmark(10);
