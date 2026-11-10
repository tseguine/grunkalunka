;; The source proof admits only these four tags. Strings retain their UTF-16 units.
(func $gc_scalar_string (param $value (ref null $gc_value)) (result (ref null $gc_value))
  (local $tag i32)
  local.get $value struct.get $gc_value 0 local.tee $tag i32.const 5 i32.eq
  if local.get $value return end
  i32.const 5 f64.const 0
  local.get $tag i32.eqz
  if (result (ref null $gc_key))
    i32.const 117 i32.const 110 i32.const 100 i32.const 101 i32.const 102 i32.const 105 i32.const 110 i32.const 101 i32.const 100 array.new_fixed $gc_key 9 ;; undefined
  else
    local.get $tag i32.const 1 i32.eq
    if (result (ref null $gc_key))
      i32.const 110 i32.const 117 i32.const 108 i32.const 108 array.new_fixed $gc_key 4 ;; null
    else
      local.get $tag i32.const 2 i32.ne if unreachable end
      local.get $value struct.get $gc_value 1 f64.const 0 f64.ne
      if (result (ref null $gc_key))
        i32.const 116 i32.const 114 i32.const 117 i32.const 101 array.new_fixed $gc_key 4 ;; true
      else
        i32.const 102 i32.const 97 i32.const 108 i32.const 115 i32.const 101 array.new_fixed $gc_key 5 ;; false
      end
    end
  end
  struct.new $gc_value)
