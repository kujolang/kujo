fn integer_mix(iterations: i64) -> i64 {
    let mut accumulator = 1_i64;
    let mut i = 1_i64;

    while i <= iterations {
        accumulator = (accumulator * 1_664_525 + i + 1_013_904_223) % 2_147_483_647;
        i += 1;
    }

    accumulator
}

fn main() {
    println!("integer_mix={}", integer_mix(250_000));
}
