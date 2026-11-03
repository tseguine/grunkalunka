;; Array.prototype.at specialized to a represented Array and a Number index.
;; Source arguments have all completed before length is read here.
(func $gc_array_at (param $array (ref null $gc_array)) (param $index f64)
  (result (ref null $gc_value)) (local $length f64)
  local.get $array struct.get $gc_array 0 f64.convert_i32_u local.set $length
  local.get $index local.get $index f64.ne
  if f64.const 0 local.set $index
  else local.get $index f64.trunc local.set $index end
  local.get $index f64.const 0 f64.lt
  if local.get $length local.get $index f64.add local.set $index end
  local.get $index f64.const 0 f64.ge
  local.get $index local.get $length f64.lt i32.and
  if (result (ref null $gc_value))
    local.get $array local.get $index i32.trunc_f64_u
    call $gc_spread_index call $gc_array_get
  else
    i32.const 0 f64.const 0 ref.null eq struct.new $gc_value
  end)
