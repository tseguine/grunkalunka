;; Number, Boolean and Null carry their numeric value in the payload.
;; Undefined converts to NaN. References are outside primitive addition.
(func $gc_scalar_number (param $input (ref null $gc_value)) (result f64)
  (local $tag i32)
  local.get $input struct.get $gc_value 0 local.tee $tag i32.const 3 i32.gt_u
  if unreachable end
  local.get $tag i32.eqz
  if (result f64) f64.const nan
  else local.get $input struct.get $gc_value 1 end)
