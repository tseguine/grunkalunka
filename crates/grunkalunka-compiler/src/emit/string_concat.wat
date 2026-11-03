;; Overflow is a private Wasm32 resource failure, not a source TypeError.
;; Checking before allocation/copy prevents a wrapped length or partial result.
(func $gc_concat_length (param $left i32) (param $right i32) (result i32)
  (local $length i64)
  local.get $left i64.extend_i32_u
  local.get $right i64.extend_i32_u i64.add local.tee $length
  i64.const 4294967295 i64.gt_u if unreachable end
  local.get $length i32.wrap_i64)

(func $gc_string_concat (param $left (ref null $gc_value))
  (param $right (ref null $gc_value)) (result (ref null $gc_value))
  (local $a (ref null $gc_key)) (local $b (ref null $gc_key))
  (local $result (ref null $gc_key))
  local.get $left struct.get $gc_value 0 i32.const 5 i32.ne
  local.get $right struct.get $gc_value 0 i32.const 5 i32.ne i32.or
  if unreachable end
  local.get $left struct.get $gc_value 2 ref.cast (ref $gc_key) local.set $a
  local.get $right struct.get $gc_value 2 ref.cast (ref $gc_key) local.set $b
  local.get $a array.len local.get $b array.len
  call $gc_concat_length
  array.new_default $gc_key local.set $result
  local.get $result i32.const 0 local.get $a i32.const 0
  local.get $a array.len array.copy $gc_key $gc_key
  local.get $result local.get $a array.len local.get $b i32.const 0
  local.get $b array.len array.copy $gc_key $gc_key
  i32.const 5 f64.const 0 local.get $result struct.new $gc_value)
