//! Share a private packed table per intrinsic prototype. Each table is created
//! once per instance and indexed by name length; a query allocates no guest keys.
use super::ir;
use crate::prototype_keys;
use std::fmt::Write;

pub(super) fn emit_runtime(module: &ir::Module, output: &mut String) {
    let mut objects = false;
    let mut arrays = false;
    for function in &module.functions {
        ir::visit_operations(&function.body, &mut |op| {
            if let ir::Op::ManagedHasProperty { input, .. } = op {
                match function.values[input.0] {
                    ir::Type::ManagedObject => objects = true,
                    ir::Type::ManagedArray => arrays = true,
                    _ => unreachable!("verified in receiver"),
                }
            }
        });
    }
    if !objects && !arrays {
        return;
    }
    output.push_str(include_str!("prototype_names.wat"));
    emit_table(output, "$object_prototype_names", prototype_keys::OBJECT);
    if objects {
        output.push_str(
            "(func $gc_object_has_property (param $obj (ref null $gc_object)) (param $key (ref null $gc_key)) (result i32)\n\
             local.get $obj local.get $key call $gc_has_own if i32.const 1 return end\n\
             local.get $key global.get $object_prototype_names call $gc_prototype_has)\n",
        );
    }
    if arrays {
        emit_table(output, "$array_prototype_names", prototype_keys::ARRAY);
        output.push_str(
            "(func $gc_array_has_property (param $array (ref null $gc_array)) (param $key (ref null $gc_key)) (result i32)\n\
             local.get $array local.get $key call $gc_array_has_own if i32.const 1 return end\n\
             local.get $key global.get $array_prototype_names call $gc_prototype_has if i32.const 1 return end\n\
             local.get $key global.get $object_prototype_names call $gc_prototype_has)\n",
        );
    }
}

fn emit_table(output: &mut String, name: &str, names: &[&str]) {
    let mut keys: Vec<Vec<u16>> = names
        .iter()
        .map(|name| name.encode_utf16().collect())
        .collect();
    keys.sort_unstable_by_key(Vec::len);
    let max = keys
        .last()
        .expect("nonempty intrinsic prototype table")
        .len();
    let mut data = vec![0_u16; max + 2];
    let mut keys = keys.into_iter().peekable();
    for length in 0..=max + 1 {
        data[length] = u16::try_from(data.len()).expect("intrinsic table fits packed offsets");
        while keys.peek().is_some_and(|key| key.len() == length) {
            data.extend(keys.next().unwrap());
        }
    }
    writeln!(output, "(global {name} (ref $gc_key)").unwrap();
    for unit in &data {
        writeln!(output, "i32.const {unit}").unwrap();
    }
    writeln!(output, "array.new_fixed $gc_key {})", data.len()).unwrap();
}
