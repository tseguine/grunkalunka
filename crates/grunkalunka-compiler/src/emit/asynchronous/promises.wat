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

;; A completed snapshot creates a fresh record with no waiters or handler.
;; Validate the same payload as settlement, without rechecking a new pointer or
;; zeroing fields immediately before overwriting them.
(func $promise_completed (param $rejected i32) (param $tag i32)
  (param $payload f64) (result i32) (local $p i32) (local $end i32)
  ;; Number payloads have no bit-pattern restriction. Keep the status check;
  ;; every other kind uses the complete canonical payload validator.
  local.get $tag i32.const 3 i32.eq
  if local.get $rejected i32.const 1 i32.gt_u if unreachable end
  else local.get $rejected local.get $tag local.get $payload call $promise_payload end
  ;; Constant-size specialization of closure_frame: same alignment, arena cap
  ;; and checked memory growth, without dynamic size/alignment checks.
  global.get $cell_top local.tee $p i32.eqz
  if i32.const 16 local.set $p end
  local.get $p i32.const 32 i32.add local.tee $end
  i32.const 1048576 i32.gt_u if unreachable end
  local.get $end memory.size i32.const 16 i32.shl i32.gt_u
  if
    local.get $end i32.const 65535 i32.add i32.const 16 i32.shr_u
    memory.size i32.sub memory.grow i32.const -1 i32.eq if unreachable end
  end
  local.get $end global.set $cell_top
  local.get $p i64.const 0x0000000150524d53 i64.store
  local.get $rejected if local.get $p i32.const 2 i32.store offset=4 end
  local.get $p local.get $tag i64.extend_i32_u i64.store offset=8
  local.get $p local.get $payload f64.store offset=16
  local.get $p i64.const 0 i64.store offset=24
  local.get $rejected
  if local.get $p i32.const 1 local.get $tag local.get $payload call $promise_event end
  local.get $p)

(func $promise_state (param $p i32) (result i32)
  local.get $p call $promise_check
  local.get $p i32.load offset=4)

(func $promise_reference (param $id f64) (result i32) (local $p i32)
  local.get $id i32.trunc_f64_u local.tee $p
  f64.convert_i32_u local.get $id f64.ne if unreachable end
  local.get $p call $promise_check
  local.get $p)

(func $promise_payload (param $rejected i32) (param $tag i32) (param $payload f64)
  local.get $rejected i32.const 1 i32.gt_u if unreachable end
  local.get $tag i32.const 5 i32.eq
  if
    local.get $rejected i32.eqz if unreachable end
    local.get $payload call $promise_reference drop return
  end
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

;; Ordered host tracking events, separate from continuation jobs. The host
;; decides which rejections remain unhandled at its microtask checkpoint.
(global $promise_event_head (mut i32) (i32.const 0))
(global $promise_event_tail (mut i32) (i32.const 0))
(func $promise_event (param $id i32) (param $kind i32)
  (param $tag i32) (param $payload f64) (local $event i32)
  call $promise_alloc local.tee $event i32.const 0x45564e54 i32.store
  local.get $event local.get $id i32.store offset=8
  local.get $event local.get $kind i32.store offset=12
  local.get $event local.get $tag i32.store offset=16
  local.get $event local.get $payload f64.store offset=24
  global.get $promise_event_tail
  if global.get $promise_event_tail local.get $event i32.store offset=4
  else local.get $event global.set $promise_event_head end
  local.get $event global.set $promise_event_tail)

;; kind 0 = empty, 1 = rejected without a handler, 2 = subsequently handled.
(func $promise_event_take (result i32 i32 i32 f64) (local $event i32)
  global.get $promise_event_head local.tee $event i32.eqz
  if i32.const 0 i32.const 0 i32.const 0 f64.const 0 return end
  local.get $event call $promise_range
  local.get $event i32.load i32.const 0x45564e54 i32.ne if unreachable end
  local.get $event i32.load offset=4 global.set $promise_event_head
  global.get $promise_event_head i32.eqz
  if i32.const 0 global.set $promise_event_tail end
  local.get $event i32.const 0 i32.store
  local.get $event i32.load offset=12
  local.get $event i32.load offset=8
  local.get $event i32.load offset=16
  local.get $event f64.load offset=24)

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
  local.get $rejected local.get $p i32.load offset=12 i32.eqz i32.and
  if local.get $p i32.const 1 local.get $tag local.get $payload call $promise_event end
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

;; Read-only host snapshot. Pending records have no completion payload.
(func $promise_snapshot (param $id f64) (result i32 i32 i32 f64)
  (local $p i32) (local $state i32) (local $tag i32) (local $payload f64)
  local.get $id call $promise_reference local.tee $p
  i32.load offset=4 local.tee $state
  if
    local.get $p call $promise_result local.set $payload local.set $tag drop
  end
  local.get $state local.get $p i32.load offset=12
  local.get $tag local.get $payload)

(func $promise_handled (param $p i32) (result i32)
  local.get $p call $promise_check
  local.get $p i32.load offset=12)

(func $promise_subscribe (param $p i32) (param $frame i32) (param $continuation i32)
  (local $waiter i32) (local $tail i32)
  local.get $p call $promise_check
  local.get $frame local.get $continuation call $reaction_new local.set $waiter
  local.get $p i32.load offset=4 i32.const 2 i32.eq
  local.get $p i32.load offset=12 i32.eqz i32.and
  if local.get $p i32.const 2 i32.const 0 f64.const 0 call $promise_event end
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
