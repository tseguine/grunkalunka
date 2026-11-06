;; Remove index zero and shift remaining own indexed entries down. Only stored
;; entries are visited; holes remain holes and named properties retain order.
(func $gc_array_shift (param $array (ref null $gc_array)) (result (ref null $gc_value))
  (local $length i32) (local $count i32) (local $index i32) (local $out i32)
  (local $position i64) (local $props (ref null $gc_props))
  (local $shifted (ref null $gc_props)) (local $entry (ref null $gc_entry))
  (local $value (ref null $gc_value))
  i32.const 0 f64.const 0 ref.null eq struct.new $gc_value local.set $value
  local.get $array struct.get $gc_array 0 local.tee $length i32.eqz
  if local.get $value return end
  local.get $array struct.get $gc_array 1 struct.get $gc_object 0 local.tee $props
  array.len local.set $count
  block $found loop $find
    local.get $index local.get $props array.len i32.ge_u br_if $found
    local.get $props local.get $index array.get $gc_props local.tee $entry
    struct.get $gc_entry 0 call $gc_key_index i64.eqz
    if
      local.get $entry struct.get $gc_entry 1 local.set $value
      local.get $count i32.const 1 i32.sub local.set $count br $found
    end
    local.get $index i32.const 1 i32.add local.set $index br $find
  end end
  local.get $count array.new_default $gc_props local.set $shifted
  i32.const 0 local.set $index
  block $done loop $copy
    local.get $index local.get $props array.len i32.ge_u br_if $done
    local.get $props local.get $index array.get $gc_props local.tee $entry
    struct.get $gc_entry 0 call $gc_key_index local.set $position
    local.get $position i64.eqz i32.eqz
    if
      local.get $shifted local.get $out
      local.get $position local.get $length i64.extend_i32_u i64.lt_u
      if (result (ref null $gc_entry))
        local.get $position i32.wrap_i64 i32.const 1 i32.sub call $gc_spread_index
        local.get $entry struct.get $gc_entry 1 struct.new $gc_entry
      else local.get $entry end
      array.set $gc_props
      local.get $out i32.const 1 i32.add local.set $out
    end
    local.get $index i32.const 1 i32.add local.set $index br $copy
  end end
  local.get $array struct.get $gc_array 1 local.get $shifted struct.set $gc_object 0
  local.get $array local.get $length i32.const 1 i32.sub struct.set $gc_array 0
  local.get $value)
