import Base

def add_mod_result(sum: U32, reduce: Bool) -> U32:
  match reduce:
    case True{}:
      U32.sub(sum, 2147483647)
    case False{}:
      sum

def add_mod(+a: U32, +b: U32) -> U32:
  +sum = U32.add(a, b)
  add_mod_result(sum, U32.is_ge(sum, 2147483647))

@unsafe def mul_mod_parity(next: U32, b: U32, result: U32, a: U32, even: Bool) -> U32:
  match even:
    case True{}:
      mul_mod(next, U32.div(b, 2), result)
    case False{}:
      mul_mod(next, U32.div(b, 2), add_mod(result, a))

@unsafe def mul_mod_zero(+a: U32, +b: U32, result: U32, zero: Bool) -> U32:
  match zero:
    case True{}:
      result
    case False{}:
      +next = add_mod(a, a)
      mul_mod_parity(next, b, result, a, U32.is_zero(U32.and(b, 1)))

@unsafe def mul_mod(+a: U32, +b: U32, result: U32) -> U32:
  mul_mod_zero(a, b, result, U32.is_zero(b))

def finish_mix_result(sum: U32, reduce: Bool) -> U32:
  match reduce:
    case True{}:
      U32.sub(sum, 2147483647)
    case False{}:
      sum

def finish_mix(product: U32, i: U32) -> U32:
  +sum = U32.add(U32.add(product, i), 1013904223)
  finish_mix_result(sum, U32.is_ge(sum, 2147483647))

@unsafe def integer_mix_bound(iterations: U32, +i: U32, accumulator: U32, in_bound: Bool) -> U32:
  match in_bound:
    case False{}:
      accumulator
    case True{}:
      +product = mul_mod(accumulator, 1664525, 0)
      integer_mix(iterations, U32.inc(i), finish_mix(product, i))

@unsafe def integer_mix(+iterations: U32, +i: U32, accumulator: U32) -> U32:
  integer_mix_bound(iterations, i, accumulator, U32.is_le(i, iterations))

def main() -> IO(Unit):
  do IO<Unit>:
    IO.print("integer_mix=" ++ U32.show(integer_mix(250000, 1, 1)))
