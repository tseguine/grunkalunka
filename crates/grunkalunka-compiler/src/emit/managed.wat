;; Private GC values preserve their language kind independently of the carrier.
;; Helpers append after source functions, preserving source function indices.
(type $gc_value (struct (field i32) (field f64) (field (ref null eq))))
;; Packed text is mutable only while its private literal is constructed.
(type $gc_key (array (mut i16)))
(type $gc_entry (struct (field (ref null $gc_key)) (field (mut (ref null $gc_value)))))
(type $gc_props (array (mut (ref null $gc_entry))))
(type $gc_object (struct (field (mut (ref null $gc_props)))))
;; Logical length is independent of sparse own-property storage.
(type $gc_array (struct (field (mut i32)) (field (ref null $gc_object))))

(func $gc_array_new (param $length i32) (result (ref null $gc_array))
  local.get $length call $gc_object_new struct.new $gc_array)

;; -1 denotes a named property. Array indices are canonical decimal keys below
;; 2^32-1; leading zeros, signs, exponent forms and non-ASCII digits stay named.
(func $gc_key_index (param $key (ref null $gc_key)) (result i64)
  (local $index i32) (local $digit i32) (local $value i64)
  local.get $key array.len i32.eqz if i64.const -1 return end
  local.get $key array.len i32.const 10 i32.gt_u if i64.const -1 return end
  local.get $key array.len i32.const 1 i32.gt_u
  if
    local.get $key i32.const 0 array.get_u $gc_key i32.const 48 i32.eq
    if i64.const -1 return end
  end
  block $done
    loop $digits
      local.get $index local.get $key array.len i32.ge_u br_if $done
      local.get $key local.get $index array.get_u $gc_key
      i32.const 48 i32.sub local.tee $digit i32.const 9 i32.gt_u
      if i64.const -1 return end
      local.get $value i64.const 10 i64.mul
      local.get $digit i64.extend_i32_u i64.add local.set $value
      local.get $index i32.const 1 i32.add local.set $index br $digits
    end
  end
  local.get $value i64.const 4294967294 i64.gt_u if i64.const -1 return end
  local.get $value)

(func $gc_length_key (param $key (ref null $gc_key)) (result i32)
  local.get $key array.len i32.const 6 i32.ne if i32.const 0 return end
  local.get $key i32.const 0 array.get_u $gc_key i32.const 108 i32.eq
  local.get $key i32.const 1 array.get_u $gc_key i32.const 101 i32.eq i32.and
  local.get $key i32.const 2 array.get_u $gc_key i32.const 110 i32.eq i32.and
  local.get $key i32.const 3 array.get_u $gc_key i32.const 103 i32.eq i32.and
  local.get $key i32.const 4 array.get_u $gc_key i32.const 116 i32.eq i32.and
  local.get $key i32.const 5 array.get_u $gc_key i32.const 104 i32.eq i32.and)

(func $gc_array_get (param $array (ref null $gc_array))
  (param $key (ref null $gc_key)) (result (ref null $gc_value))
  local.get $key call $gc_length_key
  if (result (ref null $gc_value))
    i32.const 3 local.get $array struct.get $gc_array 0 f64.convert_i32_u
    ref.null eq struct.new $gc_value
  else
    local.get $array struct.get $gc_array 1 local.get $key call $gc_get
  end)

(func $gc_array_set (param $array (ref null $gc_array))
  (param $key (ref null $gc_key)) (param $value (ref null $gc_value))
  (local $index i64)
  ;; Generic data writes cannot bypass the validated length operation.
  local.get $key call $gc_length_key if unreachable end
  local.get $key call $gc_key_index local.tee $index i64.const -1 i64.ne
  if
    local.get $index local.get $array struct.get $gc_array 0 i64.extend_i32_u i64.ge_u
    if
      local.get $array local.get $index i64.const 1 i64.add i32.wrap_i64
      struct.set $gc_array 0
    end
  end
  local.get $array struct.get $gc_array 1 local.get $key local.get $value call $gc_set)

(func $gc_array_keep (param $entry (ref null $gc_entry)) (param $length i32)
  (result i32) (local $index i64)
  local.get $entry struct.get $gc_entry 0 call $gc_key_index local.tee $index
  i64.const -1 i64.eq
  local.get $index local.get $length i64.extend_i32_u i64.lt_u i32.or)

;; Resize only accepts an already validated JS length. Storage is proportional
;; to retained properties, never to the potentially enormous logical length.
(func $gc_array_resize (param $array (ref null $gc_array)) (param $length i32)
  (local $props (ref null $gc_props)) (local $new (ref null $gc_props))
  (local $item (ref null $gc_entry)) (local $index i32) (local $count i32)
  local.get $length local.get $array struct.get $gc_array 0 i32.lt_u
  if
    local.get $array struct.get $gc_array 1 struct.get $gc_object 0 local.set $props
    block $counted
      loop $counting
        local.get $index local.get $props array.len i32.ge_u br_if $counted
        local.get $props local.get $index array.get $gc_props
        local.get $length call $gc_array_keep
        local.get $count i32.add local.set $count
        local.get $index i32.const 1 i32.add local.set $index br $counting
      end
    end
    local.get $count array.new_default $gc_props local.set $new
    i32.const 0 local.set $index i32.const 0 local.set $count
    block $copied
      loop $copying
        local.get $index local.get $props array.len i32.ge_u br_if $copied
        local.get $props local.get $index array.get $gc_props local.tee $item
        local.get $length call $gc_array_keep
        if
          local.get $new local.get $count local.get $item array.set $gc_props
          local.get $count i32.const 1 i32.add local.set $count
        end
        local.get $index i32.const 1 i32.add local.set $index br $copying
      end
    end
    local.get $array struct.get $gc_array 1 local.get $new struct.set $gc_object 0
  end
  local.get $array local.get $length struct.set $gc_array 0)

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
    local.get $left struct.get $gc_value 0 i32.const 6 i32.eq i32.or
    if (result i32)
      local.get $left struct.get $gc_value 2
      local.get $right struct.get $gc_value 2 ref.eq
    else
      local.get $left struct.get $gc_value 0 i32.const 5 i32.eq
      if (result i32)
        local.get $left struct.get $gc_value 2 ref.cast (ref $gc_key)
        local.get $right struct.get $gc_value 2 ref.cast (ref $gc_key)
        call $gc_key_equal
      else
        local.get $left struct.get $gc_value 1
        local.get $right struct.get $gc_value 1 f64.eq
      end
    end
  end)

(func $gc_truthy (param $value (ref null $gc_value)) (result i32)
  local.get $value struct.get $gc_value 0 i32.const 2 i32.lt_u
  if (result i32) i32.const 0
  else
    local.get $value struct.get $gc_value 0 i32.const 4 i32.eq
    local.get $value struct.get $gc_value 0 i32.const 6 i32.eq i32.or
    if (result i32) i32.const 1
    else
      local.get $value struct.get $gc_value 0 i32.const 5 i32.eq
      if (result i32)
        local.get $value struct.get $gc_value 2 ref.cast (ref $gc_key)
        array.len i32.const 0 i32.ne
      else
        local.get $value struct.get $gc_value 1 f64.const 0 f64.ne
        local.get $value struct.get $gc_value 1
        local.get $value struct.get $gc_value 1 f64.eq i32.and
      end
    end
  end)
