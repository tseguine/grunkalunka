;; Build exclusions one key at a time so even a long pattern has bounded operand
;; stack depth. Both literal and evaluated keys retain their UTF-16 code units.
(type $gc_rest_key (struct (field (ref null $gc_rest_key)) (field (ref null $gc_key))))

(func $gc_rest_excludes (param $keys (ref null $gc_rest_key))
  (param $key (ref null $gc_key)) (result i32)
  block $done loop $next
    local.get $keys ref.is_null br_if $done
    local.get $keys struct.get $gc_rest_key 1 local.get $key call $gc_key_equal
    if i32.const 1 return end
    local.get $keys struct.get $gc_rest_key 0 local.set $keys
    br $next
  end end
  i32.const 0)

(func $gc_object_rest (param $object (ref null $gc_object))
  (param $keys (ref null $gc_rest_key)) (result (ref null $gc_object))
  (local $source (ref null $gc_props)) (local $copy (ref null $gc_props))
  (local $entry (ref null $gc_entry)) (local $key (ref null $gc_key))
  (local $index i32) (local $count i32)
  local.get $object struct.get $gc_object 0 local.tee $source
  array.len array.new_default $gc_props local.set $copy
  block $done loop $next
    local.get $index local.get $source array.len i32.ge_u br_if $done
    local.get $source local.get $index array.get $gc_props local.tee $entry
    struct.get $gc_entry 0 local.set $key
    local.get $keys local.get $key call $gc_rest_excludes i32.eqz
    if
      ;; The slot is independent; its boxed value (including references) is shared.
      local.get $copy local.get $count
      local.get $key local.get $entry struct.get $gc_entry 1
      struct.new $gc_entry array.set $gc_props
      local.get $count i32.const 1 i32.add local.set $count
    end
    local.get $index i32.const 1 i32.add local.set $index br $next
  end end
  local.get $count local.get $source array.len i32.ne
  if
    local.get $count array.new_default $gc_props local.set $source
    local.get $source i32.const 0 local.get $copy i32.const 0 local.get $count
    array.copy $gc_props $gc_props
    local.get $source local.set $copy
  end
  local.get $copy struct.new $gc_object)
