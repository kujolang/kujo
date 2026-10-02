import Base

@unsafe def is_prime_divisible(n: U32, divisor: U32, divisible: Bool) -> Bool:
  match divisible:
    case True{}:
      False{}
    case False{}:
      is_prime(n, U32.inc(divisor))

@unsafe def is_prime_bound(+n: U32, +divisor: U32, in_bound: Bool) -> Bool:
  match in_bound:
    case False{}:
      True{}
    case True{}:
      is_prime_divisible(n, divisor, U32.is_zero(U32.mod(n, divisor)))

@unsafe def is_prime(+n: U32, +divisor: U32) -> Bool:
  is_prime_bound(n, divisor, U32.is_le(U32.mul(divisor, divisor), n))

@unsafe def count_primes_result(limit: U32, n: U32, count: U32, prime: Bool) -> U32:
  match prime:
    case True{}:
      count_primes(limit, U32.inc(n), U32.inc(count))
    case False{}:
      count_primes(limit, U32.inc(n), count)

@unsafe def count_primes_bound(limit: U32, +n: U32, count: U32, in_bound: Bool) -> U32:
  match in_bound:
    case False{}:
      count
    case True{}:
      count_primes_result(limit, n, count, is_prime(n, 2))

@unsafe def count_primes(+limit: U32, +n: U32, count: U32) -> U32:
  count_primes_bound(limit, n, count, U32.is_lt(n, limit))

def main() -> IO(Unit):
  do IO<Unit>:
    IO.print("prime_count=" ++ U32.show(count_primes(50000, 2, 0)))
