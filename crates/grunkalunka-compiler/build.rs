// Generate the normalized 125-bit powers used by the guest's binary64 formatter.
// The table is immutable WasmGC data. The generated code needs no linear memory.
use num_bigint::BigUint;
use std::{env, fmt::Write, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let mut wat = String::from("(type $ryu_table (array i64))\n");
    for (name, inverse, count) in [("pow5", false, 326), ("inv5", true, 291)] {
        writeln!(
            wat,
            "(global ${name} (ref $ryu_table) (array.new_fixed $ryu_table {}",
            2 * count
        )
        .unwrap();
        let mut power = BigUint::from(1u32);
        for _ in 0..count {
            let bits = power.bits() as usize;
            let normalized = if inverse {
                (BigUint::from(1u32) << (bits + 124)) / &power + 1u32
            } else if bits <= 125 {
                &power << (125 - bits)
            } else {
                &power >> (bits - 125)
            };
            let limbs = normalized.to_u64_digits();
            writeln!(wat, "  (i64.const {}) (i64.const {})", limbs[0], limbs[1]).unwrap();
            power *= 5u32;
        }
        wat.push_str("))\n");
    }
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("ryu-tables.wat"),
        wat,
    )
    .unwrap();
}
