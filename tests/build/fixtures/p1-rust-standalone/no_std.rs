#![no_std]
#![no_main]

// Intentionally tiny standalone native-rustc Wasm control, not the maintained
// Grunkalunka JS compiler and not the Rust Markdown component.
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop { core::hint::spin_loop(); }
}

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
pub extern "C" fn multiply_add(input: u32) -> u32 {
    input.wrapping_mul(3).wrapping_add(7)
}
