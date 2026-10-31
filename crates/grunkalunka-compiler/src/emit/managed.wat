;; Private GC values preserve their language kind independently of the carrier.
;; Helpers append after source functions, preserving source function indices.
(type $gc_value (struct (field i32) (field f64) (field (ref null eq))))
(type $gc_key (array i16))
(type $gc_entry (struct (field (ref null $gc_key)) (field (mut (ref null $gc_value)))))
(type $gc_props (array (mut (ref null $gc_entry))))
(type $gc_object (struct (field (mut (ref null $gc_props)))))

(func $gc_object_new (result (ref null $gc_object))
  i32.const 0 array.new_default $gc_props struct.new $gc_object)

(func $gc_key_equal (param $left (ref null $gc_key))
  (param $right (ref null $gc_key)) (result i32) (local $index i32)
  local.get $left array.len local.get $right array.len i32.ne
  if i32.const 0 return end
  block $same
    loop $units
      local.get $index local.get $left array.len i32.ge_u br_if $same
      local.get $left local.get $index array.get_u $gc_key
      local.get $right local.get $index array.get_u $gc_key i32.ne
      if i32.const 0 return end
      local.get $index i32.const 1 i32.add local.set $index
      br $units
    end
  end
  i32.const 1)

(func $gc_get (param $object (ref null $gc_object))
  (param $key (ref null $gc_key)) (result (ref null $gc_value))
  (local $props (ref null $gc_props)) (local $item (ref null $gc_entry))
  (local $index i32)
  local.get $object struct.get $gc_object 0 local.set $props
  block $missing
    loop $entries
      local.get $index local.get $props array.len i32.ge_u br_if $missing
      local.get $props local.get $index array.get $gc_props local.tee $item
      struct.get $gc_entry 0 local.get $key call $gc_key_equal
      if local.get $item struct.get $gc_entry 1 return end
      local.get $index i32.const 1 i32.add local.set $index
      br $entries
    end
  end
  i32.const 0 f64.const 0 ref.null eq struct.new $gc_value)

(func $gc_set (param $object (ref null $gc_object))
  (param $key (ref null $gc_key)) (param $value (ref null $gc_value))
  (local $props (ref null $gc_props)) (local $new (ref null $gc_props))
  (local $item (ref null $gc_entry)) (local $index i32)
  (local $length i32) (local $next i32)
  local.get $object struct.get $gc_object 0 local.set $props
  block $append
    loop $entries
      local.get $index local.get $props array.len i32.ge_u br_if $append
      local.get $props local.get $index array.get $gc_props local.tee $item
      struct.get $gc_entry 0 local.get $key call $gc_key_equal
      if local.get $item local.get $value struct.set $gc_entry 1 return end
      local.get $index i32.const 1 i32.add local.set $index
      br $entries
    end
  end
  local.get $props array.len local.tee $length i32.const 1 i32.add local.tee $next
  local.get $length i32.lt_u if unreachable end
  local.get $next array.new_default $gc_props local.set $new
  local.get $new i32.const 0 local.get $props i32.const 0 local.get $length
  array.copy $gc_props $gc_props
  local.get $new local.get $length local.get $key local.get $value
  struct.new $gc_entry array.set $gc_props
  local.get $object local.get $new struct.set $gc_object 0)

(func $gc_equal (param $left (ref null $gc_value))
  (param $right (ref null $gc_value)) (result i32)
  local.get $left struct.get $gc_value 0
  local.get $right struct.get $gc_value 0 i32.ne
  if (result i32) i32.const 0
  else
    local.get $left struct.get $gc_value 0 i32.const 4 i32.eq
    if (result i32)
      local.get $left struct.get $gc_value 2
      local.get $right struct.get $gc_value 2 ref.eq
    else
      local.get $left struct.get $gc_value 1
      local.get $right struct.get $gc_value 1 f64.eq
    end
  end)

(func $gc_truthy (param $value (ref null $gc_value)) (result i32)
  local.get $value struct.get $gc_value 0 i32.const 2 i32.lt_u
  if (result i32) i32.const 0
  else
    local.get $value struct.get $gc_value 0 i32.const 4 i32.eq
    if (result i32) i32.const 1
    else
      local.get $value struct.get $gc_value 1 f64.const 0 f64.ne
      local.get $value struct.get $gc_value 1
      local.get $value struct.get $gc_value 1 f64.eq i32.and
    end
  end)
