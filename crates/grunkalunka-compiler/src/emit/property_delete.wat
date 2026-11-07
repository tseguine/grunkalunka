;; Represented objects contain configurable own data properties. Removing an
;; entry preserves the remaining insertion order; a later write appends afresh.
(func $gc_delete (param $object (ref null $gc_object))
  (param $key (ref null $gc_key)) (result i32)
  (local $props (ref null $gc_props)) (local $next (ref null $gc_props))
  (local $index i32) (local $length i32)
  local.get $object struct.get $gc_object 0 local.tee $props
  array.len local.set $length
  block $missing loop $find
    local.get $index local.get $length i32.ge_u br_if $missing
    local.get $props local.get $index array.get $gc_props
    struct.get $gc_entry 0 local.get $key call $gc_key_equal
    if
      local.get $length i32.const 1 i32.sub
      array.new_default $gc_props local.set $next
      local.get $next i32.const 0 local.get $props i32.const 0
      local.get $index array.copy $gc_props $gc_props
      local.get $next local.get $index
      local.get $props local.get $index i32.const 1 i32.add
      local.get $length local.get $index i32.sub i32.const 1 i32.sub
      array.copy $gc_props $gc_props
      local.get $object local.get $next struct.set $gc_object 0
      i32.const 1 return
    end
    local.get $index i32.const 1 i32.add local.set $index br $find
  end end
  i32.const 1)
