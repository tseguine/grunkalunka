;; Numeric indexes precede other strings; insertion sort is stable for all
;; non-index keys. Sort copied entry references, never the source property table.
(func $gc_key_before (param $a (ref null $gc_entry)) (param $b (ref null $gc_entry))
  (result i32) (local $x i64) (local $y i64)
  local.get $a struct.get $gc_entry 0 call $gc_key_index local.tee $x
  i64.const -1 i64.eq if i32.const 0 return end
  local.get $b struct.get $gc_entry 0 call $gc_key_index local.tee $y
  i64.const -1 i64.eq if i32.const 1 return end
  local.get $x local.get $y i64.lt_u)

(func $gc_keys_props (param $props (ref null $gc_props)) (result (ref null $gc_array))
  (local $sorted (ref null $gc_props)) (local $out (ref null $gc_props))
  (local $entry (ref null $gc_entry)) (local $previous (ref null $gc_entry))
  (local $index i32) (local $position i32) (local $length i32)
  local.get $props array.len local.tee $length array.new_default $gc_props local.set $sorted
  local.get $sorted i32.const 0 local.get $props i32.const 0 local.get $length array.copy $gc_props $gc_props
  i32.const 1 local.set $index
  block $sorted_done loop $sort
    local.get $index local.get $length i32.ge_u br_if $sorted_done
    local.get $sorted local.get $index array.get $gc_props local.set $entry
    local.get $index local.set $position
    block $placed loop $insert
      local.get $position i32.eqz br_if $placed
      local.get $sorted local.get $position i32.const 1 i32.sub array.get $gc_props local.set $previous
      local.get $entry local.get $previous call $gc_key_before i32.eqz br_if $placed
      local.get $sorted local.get $position local.get $previous array.set $gc_props
      local.get $position i32.const 1 i32.sub local.set $position br $insert
    end end
    local.get $sorted local.get $position local.get $entry array.set $gc_props
    local.get $index i32.const 1 i32.add local.set $index br $sort
  end end
  local.get $length array.new_default $gc_props local.set $out
  i32.const 0 local.set $index
  block $copied loop $copy
    local.get $index local.get $length i32.ge_u br_if $copied
    local.get $out local.get $index
    local.get $index call $gc_spread_index
    i32.const 5 f64.const 0
    local.get $sorted local.get $index array.get $gc_props struct.get $gc_entry 0
    struct.new $gc_value struct.new $gc_entry array.set $gc_props
    local.get $index i32.const 1 i32.add local.set $index br $copy
  end end
  local.get $length local.get $out struct.new $gc_object struct.new $gc_array)

(func $gc_keys_indices (param $length i32) (result (ref null $gc_array))
  (local $out (ref null $gc_props)) (local $key (ref null $gc_key)) (local $index i32)
  local.get $length array.new_default $gc_props local.set $out
  block $done loop $next
    local.get $index local.get $length i32.ge_u br_if $done
    local.get $index call $gc_spread_index local.set $key
    local.get $out local.get $index local.get $key
    i32.const 5 f64.const 0 local.get $key struct.new $gc_value
    struct.new $gc_entry array.set $gc_props
    local.get $index i32.const 1 i32.add local.set $index br $next
  end end
  local.get $length local.get $out struct.new $gc_object struct.new $gc_array)

;; Checked specialization: nullish inputs here are private invariant failures.
(func $gc_own_keys_known (param $value (ref null $gc_value)) (result (ref null $gc_array))
  local.get $value struct.get $gc_value 0 i32.const 1 i32.le_u if unreachable end
  local.get $value struct.get $gc_value 0 i32.const 7 i32.gt_u if unreachable end
  local.get $value struct.get $gc_value 0 i32.const 4 i32.eq
  if
    local.get $value struct.get $gc_value 2 ref.test (ref $gc_array)
    if (result (ref null $gc_object))
      local.get $value struct.get $gc_value 2 ref.cast (ref $gc_array) struct.get $gc_array 1
    else
      local.get $value struct.get $gc_value 2 ref.cast (ref $gc_object)
    end
    struct.get $gc_object 0 call $gc_keys_props return
  end
  local.get $value struct.get $gc_value 0 i32.const 5 i32.eq
  if
    local.get $value struct.get $gc_value 2 ref.cast (ref $gc_key) array.len call $gc_keys_indices return
  end
  i32.const 0 call $gc_array_new)
