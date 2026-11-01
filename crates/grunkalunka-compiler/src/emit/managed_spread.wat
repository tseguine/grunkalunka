
(func $gc_object_spread (param $target (ref null $gc_object))
  (param $source (ref null $gc_value))
  (local $props (ref null $gc_props)) (local $entry (ref null $gc_entry))
  (local $text (ref null $gc_key))
  (local $index i32)
  local.get $source struct.get $gc_value 0 i32.const 7 i32.gt_u
  if unreachable end
  local.get $source struct.get $gc_value 0 i32.const 4 i32.eq
  if
    local.get $source struct.get $gc_value 2 ref.test (ref $gc_array)
    if (result (ref null $gc_object))
      local.get $source struct.get $gc_value 2 ref.cast (ref $gc_array)
      struct.get $gc_array 1
    else
      local.get $source struct.get $gc_value 2 ref.cast (ref $gc_object)
    end
    struct.get $gc_object 0 local.set $props
    block $done loop $next
      local.get $index local.get $props array.len i32.ge_u br_if $done
      local.get $props local.get $index array.get $gc_props local.set $entry
      local.get $target local.get $entry struct.get $gc_entry 0
      local.get $entry struct.get $gc_entry 1 call $gc_set
      local.get $index i32.const 1 i32.add local.set $index br $next
    end end
  end
  local.get $source struct.get $gc_value 0 i32.const 5 i32.eq
  if
    local.get $source struct.get $gc_value 2 ref.cast (ref $gc_key) local.set $text
    block $strings_done loop $characters
      local.get $index local.get $text array.len i32.ge_u br_if $strings_done
      local.get $target local.get $index call $gc_spread_index
      i32.const 5 f64.const 0
      local.get $text local.get $index array.get_u $gc_key
      array.new_fixed $gc_key 1 struct.new $gc_value call $gc_set
      local.get $index i32.const 1 i32.add local.set $index br $characters
    end end
  end)
