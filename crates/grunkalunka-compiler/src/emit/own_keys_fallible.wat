;; A semantic throw is a valid completion, unlike the checked helper's trap.
(func $own_keys_error (result f64) (local $p i32)
  call $promise_alloc local.tee $p i32.const 0x45525221 i32.store
  local.get $p i32.const 2 i32.store offset=4
  local.get $p f64.convert_i32_u)
(func $gc_try_own_keys (param $value (ref null $gc_value))
  (result (ref null $gc_array) i32 i32 f64)
  local.get $value struct.get $gc_value 0 i32.const 1 i32.le_u
  if
    ref.null $gc_array i32.const 1 i32.const 4 call $own_keys_error return
  end
  local.get $value call $gc_own_keys_known
  i32.const 0 i32.const 0 f64.const 0)
