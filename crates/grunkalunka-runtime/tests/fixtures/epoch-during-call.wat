(module
  (import "host" "tick" (func $tick))
  (global $iterations (export "iterations") (mut i32) (i32.const 0))
  (func (export "run") (result i32)
    (local $remaining i32)
    i32.const 3
    local.set $remaining
    (loop $again
      global.get $iterations
      i32.const 1
      i32.add
      global.set $iterations
      ;; Expire the deadline only after observable guest work has begun.
      global.get $iterations
      i32.const 1
      i32.eq
      if
        call $tick
      end
      local.get $remaining
      i32.const 1
      i32.sub
      local.tee $remaining
      br_if $again)
    i32.const 42))
