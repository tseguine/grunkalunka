;; Numeric endpoints have no user coercion hooks. Clamp before the Wasm32 cast.
(func $gc_array_slice_position (param $length f64) (param $number f64) (result i32)
  local.get $number local.get $number f64.ne
  if f64.const 0 local.set $number end
  local.get $number f64.trunc local.set $number
  local.get $number f64.const 0 f64.lt
  if local.get $length local.get $number f64.add local.set $number end
  local.get $number f64.const 0 f64.max local.get $length f64.min i32.trunc_f64_u)

;; Source arguments have completed. Admitted arrays have the intrinsic prototype
;; and only own data entries, so no user effects run between these two passes.
;; Count and copy stored indices in range, preserving holes without walking length.
(func $gc_array_slice (param $array (ref null $gc_array))
  (param $start f64) (param $end f64) (result (ref null $gc_array))
  (local $length f64) (local $first i32) (local $last i32)
  (local $count i32) (local $index i32) (local $out i32)
  (local $props (ref null $gc_props)) (local $copy (ref null $gc_props))
  (local $entry (ref null $gc_entry)) (local $position i64)
  local.get $array struct.get $gc_array 0 f64.convert_i32_u local.set $length
  local.get $length local.get $start call $gc_array_slice_position local.set $first
  local.get $length local.get $end call $gc_array_slice_position local.set $last
  local.get $last local.get $first i32.le_u
  if i32.const 0 call $gc_array_new return end
  local.get $array struct.get $gc_array 1 struct.get $gc_object 0 local.set $props
  block $counted loop $count_entries
    local.get $index local.get $props array.len i32.ge_u br_if $counted
    local.get $props local.get $index array.get $gc_props
    struct.get $gc_entry 0 call $gc_key_index local.set $position
    local.get $position local.get $first i64.extend_i32_u i64.ge_u
    local.get $position local.get $last i64.extend_i32_u i64.lt_u i32.and
    if local.get $count i32.const 1 i32.add local.set $count end
    local.get $index i32.const 1 i32.add local.set $index br $count_entries
  end end
  local.get $count array.new_default $gc_props local.set $copy
  i32.const 0 local.set $index
  block $copied loop $copy_entries
    local.get $index local.get $props array.len i32.ge_u br_if $copied
    local.get $props local.get $index array.get $gc_props local.tee $entry
    struct.get $gc_entry 0 call $gc_key_index local.set $position
    local.get $position local.get $first i64.extend_i32_u i64.ge_u
    local.get $position local.get $last i64.extend_i32_u i64.lt_u i32.and
    if
      local.get $copy local.get $out
      local.get $position i32.wrap_i64 local.get $first i32.sub call $gc_spread_index
      local.get $entry struct.get $gc_entry 1 struct.new $gc_entry array.set $gc_props
      local.get $out i32.const 1 i32.add local.set $out
    end
    local.get $index i32.const 1 i32.add local.set $index br $copy_entries
  end end
  local.get $last local.get $first i32.sub
  local.get $copy struct.new $gc_object struct.new $gc_array)
