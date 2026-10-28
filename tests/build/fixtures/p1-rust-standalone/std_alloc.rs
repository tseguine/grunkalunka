// Standalone native-rustc Wasm control using a small real std allocation.
// Deliberately no Cargo workspace dependencies or guest/component adapter.
#[unsafe(no_mangle)]
pub extern "C" fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let previous = b;
        b = a % previous;
        a = previous;
    }
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn sum_four(input: i32) -> i32 {
    let mut numbers = Vec::with_capacity(4);
    for offset in 0i32..4 {
        numbers.push(input.wrapping_add(offset));
    }
    numbers.into_iter().fold(0, |sum: i32, value| sum.wrapping_add(value))
}
