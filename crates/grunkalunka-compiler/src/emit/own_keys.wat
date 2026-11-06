;; Numeric indexes precede other strings; insertion sort is stable for all
;; non-index keys. Sort copied entry references, never the source property table.
(func $gc_key_before (param $a (ref null $gc_entry)) (param $b (ref null $gc_entry))
  (result i32) (local $x i64) (local $y i64)
  local.get $a struct.get $gc_entry 0 call $gc_key_index local.tee $x
  i64.const -1 i64.eq if i32.const 0 return end
  local.get $b struct.get $gc_entry 0 call $gc_key_index local.tee $y
  i64.const -1 i64.eq if i32.const 1 return end
  local.get $x local.get $y i64.lt_u)

;; kind: 0 = keys, 1 = values, 2 = fresh [key, value] pairs.
(func $gc_enumeration_item (param $kind i32) (param $key (ref null $gc_key))
  (param $value (ref null $gc_value)) (result (ref null $gc_value))
  local.get $kind i32.const 1 i32.eq if local.get $value return end
  local.get $kind i32.eqz
  if i32.const 5 f64.const 0 local.get $key struct.new $gc_value return end
  local.get $kind i32.const 2 i32.ne if unreachable end
  i32.const 4 f64.const 0 i32.const 2
  i32.const 0 call $gc_spread_index
  i32.const 5 f64.const 0 local.get $key struct.new $gc_value struct.new $gc_entry
  i32.const 1 call $gc_spread_index local.get $value struct.new $gc_entry
  array.new_fixed $gc_props 2 struct.new $gc_object struct.new $gc_array
  struct.new $gc_value)

(func $gc_enumerate_props (param $props (ref null $gc_props)) (param $kind i32) (result (ref null $gc_array))
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
    local.get $sorted local.get $index array.get $gc_props local.set $entry
    local.get $kind local.get $entry struct.get $gc_entry 0
    local.get $entry struct.get $gc_entry 1 call $gc_enumeration_item
    struct.new $gc_entry array.set $gc_props
    local.get $index i32.const 1 i32.add local.set $index br $copy
  end end
  local.get $length local.get $out struct.new $gc_object struct.new $gc_array)

(func $gc_enumerate_string (param $text (ref null $gc_key)) (param $kind i32) (result (ref null $gc_array))
  (local $out (ref null $gc_props)) (local $key (ref null $gc_key)) (local $index i32)
  (local $length i32)
  local.get $text array.len local.tee $length array.new_default $gc_props local.set $out
  block $done loop $next
    local.get $index local.get $length i32.ge_u br_if $done
    local.get $index call $gc_spread_index local.set $key
    local.get $out local.get $index local.get $key
    local.get $kind local.get $key
    local.get $kind i32.eqz
    if (result (ref null $gc_value))
      ref.null $gc_value
    else
      i32.const 5 f64.const 0 local.get $text local.get $index array.get_u $gc_key
      array.new_fixed $gc_key 1 struct.new $gc_value
    end
    call $gc_enumeration_item
    struct.new $gc_entry array.set $gc_props
    local.get $index i32.const 1 i32.add local.set $index br $next
  end end
  local.get $length local.get $out struct.new $gc_object struct.new $gc_array)

;; Checked specialization: nullish inputs here are private invariant failures.
(func $gc_enumerate_known (param $value (ref null $gc_value)) (param $kind i32) (result (ref null $gc_array))
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
    struct.get $gc_object 0 local.get $kind call $gc_enumerate_props return
  end
  local.get $value struct.get $gc_value 0 i32.const 5 i32.eq
  if
    local.get $value struct.get $gc_value 2 ref.cast (ref $gc_key) local.get $kind call $gc_enumerate_string return
  end
  i32.const 0 call $gc_array_new)
