;; The first represented object kind: an intrinsic self-resolution TypeError.
;; IDs share the invocation arena, but cannot alias Promise/reaction record kinds.
(func $error_record (param $payload f64) (result i32) (local $p i32)
  local.get $payload i32.trunc_sat_f64_u local.tee $p f64.convert_i32_u
  local.get $payload f64.ne if unreachable end
  local.get $p call $promise_range
  local.get $p i32.load i32.const 0x45525221 i32.ne if unreachable end
  local.get $p i32.load offset=4 i32.const 1 i32.ne if unreachable end
  local.get $p)
(func $type_error (result f64) (local $p i32)
  call $promise_alloc local.tee $p i32.const 0x45525221 i32.store
  local.get $p i32.const 1 i32.store offset=4
  local.get $p f64.convert_i32_u)
(func (export "error_kind") (param f64) (result i32)
  local.get 0 call $error_record i32.load offset=4)
