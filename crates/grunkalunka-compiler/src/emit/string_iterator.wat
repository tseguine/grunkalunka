;; These two helpers take an in-bounds offset into the saved UTF-16 text.
(func $gc_string_iter_width (param $text (ref null $gc_key)) (param $at i32) (result i32)
  local.get $text local.get $at array.get_u $gc_key
  i32.const 0xfc00 i32.and i32.const 0xd800 i32.eq
  if
    local.get $at i32.const 1 i32.add local.tee $at local.get $text array.len i32.lt_u
    if
      local.get $text local.get $at array.get_u $gc_key
      i32.const 0xfc00 i32.and i32.const 0xdc00 i32.eq
      if i32.const 2 return end
    end
  end
  i32.const 1)

(func $gc_string_iter_item (param $text (ref null $gc_key)) (param $at i32) (param $width i32)
  (result (ref null $gc_value))
  i32.const 5 f64.const 0
  local.get $text local.get $at array.get_u $gc_key
  local.get $width i32.const 2 i32.eq
  if (param i32) (result (ref $gc_key))
    local.get $text local.get $at i32.const 1 i32.add array.get_u $gc_key
    array.new_fixed $gc_key 2
  else
    array.new_fixed $gc_key 1
  end
  struct.new $gc_value)

(func $gc_string_iter_value (param $value (ref null $gc_value)) (param $index f64)
  (result (ref null $gc_value)) (local $text (ref null $gc_key)) (local $at i32)
  local.get $value struct.get $gc_value 0 i32.const 5 i32.ne if unreachable end
  local.get $value struct.get $gc_value 2 ref.cast (ref $gc_key) local.set $text
  local.get $index f64.const 0 f64.ge
  local.get $index local.get $text array.len f64.convert_i32_u f64.lt i32.and i32.eqz
  if i32.const 5 f64.const 0 array.new_fixed $gc_key 0 struct.new $gc_value return end
  local.get $index i32.trunc_f64_u local.set $at
  local.get $text local.get $at
  local.get $text local.get $at call $gc_string_iter_width
  call $gc_string_iter_item)
