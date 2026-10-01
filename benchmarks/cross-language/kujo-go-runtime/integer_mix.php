<?php

function integer_mix(int $iterations): int
{
    $accumulator = 1;
    $i = 1;

    while ($i <= $iterations) {
        $accumulator = ($accumulator * 1664525 + $i + 1013904223) % 2147483647;
        $i += 1;
    }

    return $accumulator;
}

echo "integer_mix=", integer_mix(250000), PHP_EOL;
