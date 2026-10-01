def count_primes(limit: int) -> int:
    count = 0
    n = 2

    while n < limit:
        is_prime = True
        divisor = 2

        while divisor * divisor <= n:
            if n % divisor == 0:
                is_prime = False
                break
            divisor += 1

        if is_prime:
            count += 1
        n += 1

    return count


print(f"prime_count={count_primes(50000)}")
