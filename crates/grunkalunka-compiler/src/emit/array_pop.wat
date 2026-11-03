;; Array.prototype.pop specialized to represented arrays with ordinary own data.
;; The receiver is saved before arguments; their effects finish before this call.
(func $gc_array_pop (param $array (ref null $gc_array))
  (result (ref null $gc_value)) (local $length i32)
  (local $value (ref null $gc_value))
  local.get $array struct.get $gc_array 0 local.tee $length i32.eqz
  if
    i32.const 0 f64.const 0 ref.null eq struct.new $gc_value return
  end
  local.get $length i32.const 1 i32.sub local.set $length
  local.get $array local.get $length call $gc_spread_index call $gc_array_get
  local.set $value
  local.get $array local.get $length call $gc_array_resize
  local.get $value)
