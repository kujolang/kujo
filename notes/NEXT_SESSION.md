# Archived Historical Note (Superseded)

This file captures a **January 2026 v0.9.0-era performance plan** and is kept for historical context only.

It is **not** an active release gate for current Kujo work.

Active planning and release authority now lives in:

1. `ROADMAP.md` (current execution and release checklist)
2. `docs/V1_SCOPE.md` (v1 in-scope/out-of-scope commitments)
3. `docs/RELEASE_PROCESS.md` (release procedure and policy)

---

# 🎯 NEXT SESSION: Make Kujo FASTER Than Python! (Historical)

**Date Created:** 2026-01-28  
**Priority:** 🔥 CRITICAL - BLOCKING v0.9.0 RELEASE  
**Estimated Time:** 1-2 weeks

---

## 🚨 Current Status

### ✅ What Works (JIT Execution Success!)
- **Array Sum (1M elements)**: 52ms - **MATCHES PYTHON!** 🎯
- **Hash Map (100k items)**: 34ms - **MATCHES PYTHON!** 🎯
- JIT compiler fully functional and executing native code
- 239x speedup on pure computational loops

### ❌ What's Broken (MUST FIX!)
- **Fibonacci Recursive (n=30)**: 11,782ms vs Python 282ms - **42x SLOWER!** 😱
- **Fibonacci Iterative (100k)**: 918ms vs Python 118ms - **7.8x SLOWER!** 😱

---

## 🎯 Mission for Next Session

**MAKE KUJO 5-10x FASTER THAN PYTHON ACROSS ALL BENCHMARKS**

This is non-negotiable. Kujo's viability as a language depends on performance.

---

## 🔍 Root Causes Identified

1. **JIT Coverage Too Limited**
   - Only handles pure integer arithmetic loops
   - Falls back to interpretation for everything else
   
2. **Function Calls Not JIT-Compiled**
   - Every function call goes through slow interpreter path
   - No inline caching or type feedback
   
3. **String Constants Block Compilation**
   - Print statements prevent JIT from engaging
   - Entire compilation fails on single string
   
4. **No Recursive Optimization**
   - Each recursive call interpreted from scratch
   - No memoization or tail call optimization

---

## 📋 Implementation Plan (Priority Order)

### Week 1: Expand JIT Coverage (CRITICAL)

**1. String Constant Handling** (Day 1-2)
- [ ] Skip/stub print operations in JIT code
- [ ] Allow compilation even with strings present
- [ ] Don't fail entire compilation for single print
- **Impact:** Fibonacci and other functions can now be JIT-compiled

**2. Function Call Support** (Day 3-4)
- [ ] JIT-compile function entry/exit points
- [ ] Inline hot functions (>1000 calls)
- [ ] JIT-to-JIT transitions (compiled → compiled calls)
- [ ] Guard on function identity for polymorphic calls
- **Impact:** 10-50x speedup on function-heavy code

**3. Return Value Optimization** (Day 5)
- [ ] Fast path for integer returns
- [ ] Avoid boxing/unboxing for primitives
- [ ] Direct register passing
- **Impact:** 2-5x speedup on function returns

### Week 2: Fibonacci-Specific Optimizations

**4. Recursive Call Inlining** (Day 6-7)
- [ ] Detect recursive patterns (fib(n-1) + fib(n-2))
- [ ] Generate specialized code with memoization
- [ ] Inline small recursive calls
- **Impact:** 10-20x speedup for recursive fibonacci

**5. Iterative Loop Optimization** (Day 8-9)
- [ ] Better loop variable tracking
- [ ] Recognize accumulator patterns
- [ ] Eliminate redundant stores
- **Impact:** 5-10x speedup for iterative fibonacci

**6. Type Feedback** (Day 10)
- [ ] Guard that n is Int at function entry
- [ ] Eliminate type checks in hot path
- [ ] Specialize on integer arguments
- **Impact:** 2-3x additional speedup

---

## 🎯 Performance Targets (Non-Negotiable)

```
CURRENT STATE:
- Fib Recursive (n=30):  11,782ms (Kujo) vs 282ms (Python) - 42x slower ❌
- Fib Iterative (100k):     918ms (Kujo) vs 118ms (Python) - 7.8x slower ❌

TARGET AFTER FIXES:
- Fib Recursive (n=30):  <50ms  (5-10x FASTER than Python) ✅
- Fib Iterative (100k):  <20ms  (5-10x FASTER than Python) ✅
- Array Sum (1M):        <10ms  (5x FASTER than Python) ✅
- Hash Map (100k):       <20ms  (still faster than Python) ✅

STRETCH GOAL:
- Match or exceed Go performance across all benchmarks
```

---

## 📊 How to Test Progress

Run benchmarks after each fix:
```bash
cd /Users/robertdevore/Documents/Kujolang/kujo-repos/kujo
chmod +x test_jit_loop.sh
./test_jit_loop.sh

# Or run full suite
cd benchmarks/cross-language
./run_benchmarks.sh

# Check latest results
cat results/benchmark_*.txt | tail -100
```

Expected output progression:
1. After string handling: Fib recursive ~5,000ms (2x faster)
2. After function calls: Fib recursive ~1,000ms (10x faster)
3. After inlining: Fib recursive ~50ms (200x faster - GOAL!)

---

## 🔧 Key Files to Modify

- **src/jit.rs** (lines 719-742): LoadConst handling - add string support
- **src/jit.rs** (lines 913-917): Add function call opcodes (Call, Return, CallNative)
- **src/jit.rs** (new): Add recursive pattern detection
- **src/vm.rs** (lines 197-273): May need adjustments for better JIT triggering
- **src/bytecode.rs**: May need new opcodes for optimized paths

---

## 💡 Debugging Tips

If JIT fails to compile:
```bash
DEBUG_JIT=1 ./target/release/kujo run test.kujo 2>&1 | grep -E "(JIT|compile)"
```

Check what opcodes are causing failures:
```bash
DEBUG_JIT=1 ./target/release/kujo run test.kujo 2>&1 | grep "Unsupported"
```

Profile to see interpretation vs JIT ratio:
```bash
cargo run --release -- profile benchmarks/cross-language/bench.kujo
```

---

## ⚠️ Success Criteria (BLOCKING v0.9.0)

v0.9.0 **CANNOT** ship until:
- ✅ All benchmarks faster than Python (minimum 2x, target 5-10x)
- ✅ Fibonacci recursive within 10x of Go performance
- ✅ Fibonacci iterative within 5x of Go performance
- ✅ JIT compilation ratio >80% for hot code
- ✅ Zero correctness regressions

---

## 📚 Reference Documents

- **ROADMAP.md**: Phase 7 detailed plan
- **notes/2026-01-28_04-08_jit-execution-success.md**: Current state analysis
- **notes/2026-01-28_03-40_fixes-applied.md**: What we fixed this session
- **benchmarks/cross-language/RESULTS.md**: Performance comparisons
- **src/jit.rs**: JIT compiler implementation (lines 650-1100)

---

## 🚀 Expected Outcome

After completing Phase 7:
- **Kujo will be 5-10x FASTER than Python** ✅
- **Kujo will approach Go performance** ✅
- **v0.9.0 can ship with confidence** ✅
- **Kujo will be a viable high-performance language** ✅

---

## 🎬 Getting Started Next Session

1. **Read this document first** - understand the mission
2. **Review ROADMAP.md Phase 7** - detailed technical plan
3. **Run current benchmarks** - establish baseline
4. **Start with string constant handling** - quickest win
5. **Test after each change** - verify progress
6. **Commit frequently** - track improvements

**Remember:** This is CRITICAL. Kujo's success depends on being faster than Python. No compromises!

---

**Good luck! 🚀**

The JIT infrastructure is solid. We just need to expand coverage. You've got this!
