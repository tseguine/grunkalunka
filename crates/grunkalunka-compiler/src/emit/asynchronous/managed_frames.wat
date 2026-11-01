;; Frame addresses are aligned offsets in the existing bounded invocation arena.
;; They select roots; no GC reference is stored in that arena.
(type $async_saved_refs (array (mut eqref)))
(type $async_root (struct (field i32) (field (ref null $gc_frame))
                         (field (ref $async_saved_refs))))
(type $async_roots (array (mut (ref null $async_root))))
(global $async_root_map (mut (ref null $async_roots)) (ref.null $async_roots))

(func $async_root_index (param $address i32) (result i32)
  local.get $address i32.const 1048576 i32.ge_u if unreachable end
  local.get $address i32.const 15 i32.and if unreachable end
  local.get $address global.get $cell_top i32.ge_u if unreachable end
  local.get $address i32.const 4 i32.shr_u)

(func $async_roots_get (param $address i32) (param $owner i32)
  (result (ref $async_root)) (local $entry (ref null $async_root))
  global.get $async_root_map local.get $address call $async_root_index
  array.get $async_roots local.tee $entry
  struct.get $async_root 0 local.get $owner i32.ne if unreachable end
  local.get $entry ref.as_non_null)

(func $async_roots_new (param $address i32) (param $owner i32)
  (param $count i32) (param $environment (ref null $gc_frame))
  (local $index i32) (local $old i32) (local $size i32)
  (local $grown (ref null $async_roots))
  ;; One reference lane cannot exceed the existing arena's eight-byte slot cap.
  local.get $count i32.const 131072 i32.gt_u if unreachable end
  local.get $address call $async_root_index local.set $index
  global.get $async_root_map ref.is_null
  if i32.const 0 local.set $old
  else global.get $async_root_map array.len local.set $old end
  local.get $index local.get $old i32.ge_u
  if
    local.get $old local.set $size
    local.get $size i32.eqz if i32.const 16 local.set $size end
    block $large_enough
      loop $grow
        local.get $index local.get $size i32.lt_u br_if $large_enough
        local.get $size i32.const 2 i32.mul local.set $size
        br $grow
      end
    end
    local.get $size array.new_default $async_roots local.set $grown
    local.get $old
    if
      local.get $grown i32.const 0 global.get $async_root_map i32.const 0
      local.get $old array.copy $async_roots $async_roots
    end
    local.get $grown global.set $async_root_map
  end
  global.get $async_root_map local.get $index array.get $async_roots
  ref.is_null i32.eqz if unreachable end
  global.get $async_root_map local.get $index
  local.get $owner local.get $environment
  local.get $count array.new_default $async_saved_refs
  struct.new $async_root array.set $async_roots)

(func $async_roots_drop (param $address i32) (param $owner i32)
  local.get $address local.get $owner call $async_roots_get drop
  global.get $async_root_map local.get $address call $async_root_index
  ref.null $async_root array.set $async_roots)
