;; Admitted arrays have intrinsic iteration and only own data entries. No source
;; effect can run while the remainder is consumed, so length is captured once.
;; Allocate fresh dense entries, then overlay original indexed values. This is
;; linear in the resulting length plus the source's stored property count.
(func $gc_array_rest (param $array (ref null $gc_array)) (param $start i32)
  (result (ref null $gc_array))
  (local $length i32) (local $count i32) (local $index i32)
  (local $props (ref null $gc_props)) (local $dense (ref null $gc_props))
  (local $entry (ref null $gc_entry)) (local $position i64)
  (local $undefined (ref null $gc_value))
  local.get $array struct.get $gc_array 0 local.set $length
  local.get $start local.get $length i32.ge_u
  if i32.const 0 call $gc_array_new return end
  local.get $length local.get $start i32.sub local.tee $count
  array.new_default $gc_props local.set $dense
  i32.const 0 f64.const 0 ref.null eq struct.new $gc_value local.set $undefined
  block $filled
    loop $fill
      local.get $index local.get $count i32.ge_u br_if $filled
      local.get $dense local.get $index
      local.get $index call $gc_spread_index local.get $undefined
      struct.new $gc_entry array.set $gc_props
      local.get $index i32.const 1 i32.add local.set $index br $fill
    end
  end
  local.get $array struct.get $gc_array 1 struct.get $gc_object 0 local.set $props
  i32.const 0 local.set $index
  block $copied
    loop $copy
      local.get $index local.get $props array.len i32.ge_u br_if $copied
      local.get $props local.get $index array.get $gc_props local.tee $entry
      struct.get $gc_entry 0 call $gc_key_index local.set $position
      local.get $position local.get $start i64.extend_i32_u i64.ge_u
      local.get $position local.get $length i64.extend_i32_u i64.lt_u i32.and
      if
        local.get $dense
        local.get $position i32.wrap_i64 local.get $start i32.sub
        array.get $gc_props
        local.get $entry struct.get $gc_entry 1 struct.set $gc_entry 1
      end
      local.get $index i32.const 1 i32.add local.set $index br $copy
    end
  end
  local.get $count local.get $dense struct.new $gc_object struct.new $gc_array)
