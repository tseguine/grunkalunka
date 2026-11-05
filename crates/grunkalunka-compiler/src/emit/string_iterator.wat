;; The loop supplies an in-bounds UTF-16 offset. One iteration yields a
;; surrogate pair together, or exactly one code unit in every other case.
(func $gc_string_iter_value (param $value (ref null $gc_value)) (param $index f64)
  (result (ref null $gc_value))
  (local $text (ref null $gc_key)) (local $at i32) (local $first i32) (local $second i32)
  local.get $value struct.get $gc_value 0 i32.const 5 i32.ne if unreachable end
  local.get $value struct.get $gc_value 2 ref.cast (ref $gc_key) local.set $text
  local.get $index f64.const 0 f64.ge
  local.get $index local.get $text array.len f64.convert_i32_u f64.lt i32.and i32.eqz
  if
    i32.const 5 f64.const 0 array.new_fixed $gc_key 0 struct.new $gc_value return
  end
  local.get $index i32.trunc_f64_u local.set $at
  local.get $text local.get $at array.get_u $gc_key local.tee $first
  i32.const 0xfc00 i32.and i32.const 0xd800 i32.eq
  if
    local.get $at i32.const 1 i32.add local.tee $at local.get $text array.len i32.lt_u
    if
      local.get $text local.get $at array.get_u $gc_key local.tee $second
      i32.const 0xfc00 i32.and i32.const 0xdc00 i32.eq
      if
        i32.const 5 f64.const 0 local.get $first local.get $second
        array.new_fixed $gc_key 2 struct.new $gc_value return
      end
    end
  end
  i32.const 5 f64.const 0 local.get $first array.new_fixed $gc_key 1 struct.new $gc_value)
