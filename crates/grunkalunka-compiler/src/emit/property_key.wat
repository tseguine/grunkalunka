;; A private ToPropertyKey subset for boxed runtime values. Guest-owned
;; normalization preserves JavaScript primitive/String key spelling. Objects,
;; functions and Symbols require a later ToPrimitive/completion operation.
(func $gc_property_key (param $input (ref null $gc_value)) (result (ref $gc_key))
  (local $tag i32)
  local.get $input struct.get $gc_value 0 local.tee $tag
  i32.const 5 i32.eq
  if
    local.get $input struct.get $gc_value 2 ref.cast (ref $gc_key)
    return
  end
  local.get $tag i32.const 3 i32.le_u
  if
    local.get $input call $gc_primitive_string
    struct.get $gc_value 2 ref.cast (ref $gc_key)
    return
  end
  unreachable)
