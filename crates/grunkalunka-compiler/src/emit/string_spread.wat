;; Count code points to allocate exactly the required slots. The second pass
;; fills the fresh property table directly, including unpaired surrogates.
(func $gc_string_to_array (param $value (ref null $gc_value)) (result (ref null $gc_array))
  (local $text (ref null $gc_key)) (local $length i32) (local $at i32)
  (local $count i32) (local $index i32) (local $width i32) (local $props (ref null $gc_props))
  local.get $value struct.get $gc_value 0 i32.const 5 i32.ne if unreachable end
  local.get $value struct.get $gc_value 2 ref.cast (ref $gc_key) local.tee $text array.len local.set $length
  block $counted loop $count_next
    local.get $at local.get $length i32.ge_u br_if $counted
    local.get $at local.get $text local.get $at call $gc_string_iter_width i32.add local.set $at
    local.get $count i32.const 1 i32.add local.set $count br $count_next
  end end
  local.get $count array.new_default $gc_props local.set $props
  i32.const 0 local.set $at
  block $filled loop $fill
    local.get $index local.get $count i32.ge_u br_if $filled
    local.get $text local.get $at call $gc_string_iter_width local.set $width
    local.get $props local.get $index
    local.get $index call $gc_spread_index
    local.get $text local.get $at local.get $width call $gc_string_iter_item
    struct.new $gc_entry array.set $gc_props
    local.get $at local.get $width i32.add local.set $at
    local.get $index i32.const 1 i32.add local.set $index br $fill
  end end
  local.get $count local.get $props struct.new $gc_object struct.new $gc_array)
