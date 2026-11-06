;; Own-data intrinsic arrays have no accessors or proxy hooks. Rebuild indexed
;; keys in one pass over stored properties; named entries and length stay intact.
;; The backing object is private, while the Array reference returned is unchanged.
(func $gc_array_reverse (param $array (ref null $gc_array)) (result (ref null $gc_array))
  (local $length i32) (local $index i32) (local $position i64)
  (local $props (ref null $gc_props)) (local $reversed (ref null $gc_props))
  (local $entry (ref null $gc_entry))
  local.get $array struct.get $gc_array 0 local.set $length
  local.get $array struct.get $gc_array 1 struct.get $gc_object 0 local.tee $props
  array.len array.new_default $gc_props local.set $reversed
  block $done loop $copy
    local.get $index local.get $props array.len i32.ge_u br_if $done
    local.get $props local.get $index array.get $gc_props local.tee $entry
    struct.get $gc_entry 0 call $gc_key_index local.set $position
    local.get $reversed local.get $index
    local.get $position local.get $length i64.extend_i32_u i64.lt_u
    if (result (ref null $gc_entry))
      local.get $length i32.const 1 i32.sub local.get $position i32.wrap_i64 i32.sub
      call $gc_spread_index local.get $entry struct.get $gc_entry 1 struct.new $gc_entry
    else local.get $entry end
    array.set $gc_props
    local.get $index i32.const 1 i32.add local.set $index br $copy
  end end
  local.get $array struct.get $gc_array 1 local.get $reversed struct.set $gc_object 0
  local.get $array)
