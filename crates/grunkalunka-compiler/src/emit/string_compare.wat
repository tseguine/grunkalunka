;; Return -1, 0 or 1 using unsigned UTF-16 units, then length for equal prefixes.
;; Runtime tag checks defend the known-String IR specialization.
(func $gc_string_compare (param $left (ref null $gc_value))
  (param $right (ref null $gc_value)) (result i32)
  (local $a (ref null $gc_key)) (local $b (ref null $gc_key))
  (local $a_length i32) (local $b_length i32) (local $index i32)
  (local $a_unit i32) (local $b_unit i32)
  local.get $left struct.get $gc_value 0 i32.const 5 i32.ne
  local.get $right struct.get $gc_value 0 i32.const 5 i32.ne i32.or
  if unreachable end
  local.get $left struct.get $gc_value 2 ref.cast (ref $gc_key) local.tee $a
  array.len local.set $a_length
  local.get $right struct.get $gc_value 2 ref.cast (ref $gc_key) local.tee $b
  array.len local.set $b_length
  block $prefix
    loop $units
      local.get $index local.get $a_length i32.ge_u
      local.get $index local.get $b_length i32.ge_u i32.or br_if $prefix
      local.get $a local.get $index array.get_u $gc_key local.set $a_unit
      local.get $b local.get $index array.get_u $gc_key local.set $b_unit
      local.get $a_unit local.get $b_unit i32.lt_u
      if i32.const -1 return end
      local.get $a_unit local.get $b_unit i32.gt_u
      if i32.const 1 return end
      local.get $index i32.const 1 i32.add local.set $index
      br $units
    end
  end
  local.get $a_length local.get $b_length i32.lt_u
  if i32.const -1 return end
  local.get $a_length local.get $b_length i32.gt_u)
