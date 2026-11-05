(func $gc_has_own (param $object (ref null $gc_object))
  (param $key (ref null $gc_key)) (result i32)
  (local $props (ref null $gc_props)) (local $index i32)
  local.get $object struct.get $gc_object 0 local.set $props
  block $missing loop $next
    local.get $index local.get $props array.len i32.ge_u br_if $missing
    local.get $props local.get $index array.get $gc_props
    struct.get $gc_entry 0 local.get $key call $gc_key_equal
    if i32.const 1 return end
    local.get $index i32.const 1 i32.add local.set $index br $next
  end end
  i32.const 0)

(func $gc_array_has_own (param $array (ref null $gc_array))
  (param $key (ref null $gc_key)) (result i32)
  local.get $key call $gc_length_key if i32.const 1 return end
  local.get $array struct.get $gc_array 1 local.get $key call $gc_has_own)
