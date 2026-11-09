;; Source Array.prototype.push: append after arguments complete. The 32-bit
;; length limit must not wrap into an indexed property of zero.
(func $gc_array_push (param $array (ref null $gc_array))
  (param $value (ref null $gc_value)) (result f64)
  (local $length i32)
  local.get $array struct.get $gc_array 0 local.tee $length
  i32.const -1 i32.eq if unreachable end
  local.get $array
  local.get $length call $gc_spread_index
  local.get $value
  call $gc_array_set
  local.get $array struct.get $gc_array 0
  f64.convert_i32_u)
