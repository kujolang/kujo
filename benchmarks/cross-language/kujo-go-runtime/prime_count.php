<?php

function count_primes(int $limit): int
{
    $count = 0;
    $n = 2;

    while ($n < $limit) {
        $is_prime = true;
        $divisor = 2;

        while ($divisor * $divisor <= $n) {
            if ($n % $divisor === 0) {
                $is_prime = false;
                break;
            }
            $divisor += 1;
        }

        if ($is_prime) {
            $count += 1;
        }
        $n += 1;
    }

    return $count;
}

echo "prime_count=", count_primes(50000), PHP_EOL;
