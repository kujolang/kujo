// Array Operations Comparison Benchmark for Node.js
// Tests: map, filter, reduce operations on large arrays

function benchmarkArrayOps(size) {
    console.log(`Array size: ${size}`);
    
    // Create array
    const arr = Array.from({length: size}, (_, i) => i);
    
    // Map: double each element
    let start = Date.now();
    const doubled = arr.map(x => x * 2);
    const mapTime = Date.now() - start;
    console.log(`Map time: ${mapTime}ms`);
    
    // Filter: keep only evens
    start = Date.now();
    const evens = arr.filter(x => x % 2 === 0);
    const filterTime = Date.now() - start;
    console.log(`Filter time: ${filterTime}ms`);
    
    // Reduce: sum all elements
    start = Date.now();
    const sum = arr.reduce((acc, x) => acc + x, 0);
    const reduceTime = Date.now() - start;
    console.log(`Reduce time: ${reduceTime}ms`);
    
    const total = mapTime + filterTime + reduceTime;
    console.log(`Total time: ${total}ms`);
    console.log(`Sum: ${sum}`);
}

console.log("Node.js Array Operations Benchmark");
console.log("===================================");
benchmarkArrayOps(100000);
