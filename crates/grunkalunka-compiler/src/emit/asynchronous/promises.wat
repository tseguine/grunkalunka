;; Private guest helper. The caller supplies memory, $cell_top and $closure_frame.
;; Promise: magic/state/tag/handled/payload/waiter-head/waiter-tail (32 bytes).
;; Reaction: magic/next/frame/continuation/tag/rejected/payload (32 bytes).
(func $promise_alloc (result i32) (local $p i32)
  ;; Other arena users may own address zero; only Promise/queue links reserve it.
  global.get $cell_top i32.eqz
  if i32.const 16 call $closure_frame drop end
  i32.const 32 call $closure_frame local.set $p
  local.get $p i64.const 0 i64.store
  local.get $p i64.const 0 i64.store offset=8
  local.get $p i64.const 0 i64.store offset=16
  local.get $p i64.const 0 i64.store offset=24
  local.get $p)

(func $promise_range (param $p i32)
  local.get $p i32.const 16 i32.lt_u if unreachable end
  local.get $p i32.const 15 i32.and if unreachable end
  local.get $p global.get $cell_top i32.gt_u if unreachable end
  global.get $cell_top local.get $p i32.sub i32.const 32 i32.lt_u
  if unreachable end)

(func $promise_check (param $p i32)
  local.get $p call $promise_range
  local.get $p i32.load i32.const 0x50524d53 i32.ne if unreachable end
  local.get $p i32.load offset=4 i32.const 2 i32.gt_u if unreachable end
  local.get $p i32.load offset=12 i32.const 1 i32.gt_u if unreachable end)

(func $promise_new (result i32) (local $p i32)
  call $promise_alloc local.tee $p i32.const 0x50524d53 i32.store
  local.get $p)

(func $promise_state (param $p i32) (result i32)
  local.get $p call $promise_check
  local.get $p i32.load offset=4)

(func $promise_payload (param $rejected i32) (param $tag i32) (param $payload f64)
  local.get $rejected i32.const 1 i32.gt_u if unreachable end
  local.get $tag i32.const 3 i32.gt_u if unreachable end
  local.get $tag i32.const 3 i32.eq if return end
  local.get $tag i32.const 2 i32.eq
  if
    local.get $payload i64.reinterpret_f64 i64.eqz
    local.get $payload f64.const 1 f64.eq i32.or
    i32.eqz if unreachable end
  else
    local.get $payload i64.reinterpret_f64 i64.const 0 i64.ne
    if unreachable end
  end)

(global $reaction_head (mut i32) (i32.const 0))
(global $reaction_tail (mut i32) (i32.const 0))
(global $reaction_count (mut i32) (i32.const 0))
(global $reaction_new_count (mut i32) (i32.const 0))

(func $reaction_new (param $frame i32) (param $continuation i32) (result i32)
  (local $p i32)
  call $promise_alloc local.tee $p i32.const 0x57414954 i32.store
  local.get $p local.get $frame i32.store offset=8
  local.get $p local.get $continuation i32.store offset=12
  local.get $p)

;; Enqueue exactly once. Pending waiter links are consumed before this call.
(func $reaction_enqueue (param $p i32)
  local.get $p call $promise_range
  local.get $p i32.load i32.const 0x57414954 i32.ne if unreachable end
  local.get $p i32.const 0x4a4f4251 i32.store
  local.get $p i32.const 0 i32.store offset=4
  global.get $reaction_tail
  if
    global.get $reaction_tail local.get $p i32.store offset=4
  else
    local.get $p global.set $reaction_head
  end
  local.get $p global.set $reaction_tail
  global.get $reaction_count i32.const 1 i32.add global.set $reaction_count
  global.get $reaction_new_count i32.const 1 i32.add global.set $reaction_new_count)

(func $promise_settle (param $p i32) (param $rejected i32)
  (param $tag i32) (param $payload f64) (result i32)
  (local $waiter i32) (local $next i32)
  local.get $p call $promise_check
  local.get $rejected local.get $tag local.get $payload call $promise_payload
  local.get $p i32.load offset=4 if i32.const 0 return end
  local.get $p local.get $rejected i32.const 1 i32.add i32.store offset=4
  local.get $p local.get $tag i32.store offset=8
  local.get $p local.get $payload f64.store offset=16
  local.get $p i32.load offset=24 local.set $waiter
  local.get $p i32.const 0 i32.store offset=24
  local.get $p i32.const 0 i32.store offset=28
  block $done
    loop $waiters
      local.get $waiter i32.eqz br_if $done
      local.get $waiter call $promise_range
      local.get $waiter i32.load i32.const 0x57414954 i32.ne if unreachable end
      local.get $waiter i32.load offset=4 local.set $next
      local.get $waiter local.get $tag i32.store offset=16
      local.get $waiter local.get $rejected i32.store offset=20
      local.get $waiter local.get $payload f64.store offset=24
      local.get $waiter call $reaction_enqueue
      local.get $next local.set $waiter
      br $waiters
    end
  end
  i32.const 1)

(func $promise_result (param $p i32) (result i32 i32 f64)
  local.get $p call $promise_check
  local.get $p i32.load offset=4 i32.eqz if unreachable end
  local.get $p i32.load offset=4 i32.const 2 i32.eq
  local.get $p i32.load offset=8
  local.get $p f64.load offset=16 call $promise_payload
  local.get $p i32.load offset=4 i32.const 2 i32.eq
  local.get $p i32.load offset=8
  local.get $p f64.load offset=16)

(func $promise_handled (param $p i32) (result i32)
  local.get $p call $promise_check
  local.get $p i32.load offset=12)

(func $promise_subscribe (param $p i32) (param $frame i32) (param $continuation i32)
  (local $waiter i32) (local $tail i32)
  local.get $p call $promise_check
  local.get $frame local.get $continuation call $reaction_new local.set $waiter
  local.get $p i32.const 1 i32.store offset=12
  local.get $p i32.load offset=4 i32.eqz
  if
    local.get $p i32.load offset=28 local.tee $tail
    if local.get $tail local.get $waiter i32.store offset=4
    else local.get $p local.get $waiter i32.store offset=24 end
    local.get $p local.get $waiter i32.store offset=28
  else
    local.get $waiter local.get $p i32.load offset=8 i32.store offset=16
    local.get $waiter local.get $p i32.load offset=4 i32.const 2 i32.eq i32.store offset=20
    local.get $waiter local.get $p f64.load offset=16 f64.store offset=24
    local.get $waiter call $reaction_enqueue
  end)

;; Direct primitive awaits use the same queue as Promise reactions.
(func $reaction_schedule (param $frame i32) (param $continuation i32)
  (param $rejected i32) (param $tag i32) (param $payload f64) (local $p i32)
  local.get $rejected local.get $tag local.get $payload call $promise_payload
  local.get $frame local.get $continuation call $reaction_new local.set $p
  local.get $p local.get $tag i32.store offset=16
  local.get $p local.get $rejected i32.store offset=20
  local.get $p local.get $payload f64.store offset=24
  local.get $p call $reaction_enqueue)

(func $reaction_ready (result i32) global.get $reaction_count)

;; A host must schedule this many individual callbacks at this enqueue boundary.
(func $reaction_issued (result i32) (local $count i32)
  global.get $reaction_new_count local.set $count
  i32.const 0 global.set $reaction_new_count
  local.get $count)

(func $reaction_take (result i32) (local $p i32)
  global.get $reaction_head local.tee $p i32.eqz if i32.const 0 return end
  local.get $p call $promise_range
  local.get $p i32.load i32.const 0x4a4f4251 i32.ne if unreachable end
  global.get $reaction_count i32.eqz if unreachable end
  local.get $p i32.load offset=4 global.set $reaction_head
  global.get $reaction_head i32.eqz if i32.const 0 global.set $reaction_tail end
  global.get $reaction_count i32.const 1 i32.sub global.set $reaction_count
  local.get $p i32.const 0 i32.store offset=4
  local.get $p i32.const 0x4a4f4244 i32.store
  local.get $p)

;; Reading a consumed queue record is pure; only take performs delivery.
(func $reaction_read (param $p i32) (result i32 i32 i32 i32 f64)
  local.get $p call $promise_range
  local.get $p i32.load i32.const 0x4a4f4244 i32.ne if unreachable end
  local.get $p i32.load offset=20
  local.get $p i32.load offset=16
  local.get $p f64.load offset=24 call $promise_payload
  local.get $p i32.load offset=8
  local.get $p i32.load offset=12
  local.get $p i32.load offset=20
  local.get $p i32.load offset=16
  local.get $p f64.load offset=24)
