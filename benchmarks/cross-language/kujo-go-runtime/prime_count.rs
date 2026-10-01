fn count_primes(limit: i64) -> i64 {
    let mut count = 0;
    let mut n = 2;

    while n < limit {
        let mut is_prime = true;
        let mut divisor = 2;

        while divisor * divisor <= n {
            if n % divisor == 0 {
                is_prime = false;
                break;
            }
            divisor += 1;
        }

        if is_prime {
            count += 1;
        }
        n += 1;
    }

    count
}

fn main() {
    println!("prime_count={}", count_primes(50_000));
}
