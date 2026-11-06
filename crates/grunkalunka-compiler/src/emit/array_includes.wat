;; The represented Array has own data properties and no indexed prototype
;; hooks. Check each present slot once; the remaining range consists of holes.
(func $gc_array_includes (param $array (ref null $gc_array)) (param $needle (ref null $gc_value))
  (param $from f64) (result i32)
  (local $length i32) (local $start i32) (local $props (ref null $gc_props))
  (local $slot i32) (local $present i32) (local $key i64) (local $entry (ref null $gc_entry))
  (local $item (ref null $gc_value)) (local $needle_nan i32)
  local.get $array struct.get $gc_array 0 local.tee $length i32.eqz if i32.const 0 return end
  local.get $from local.get $from f64.ne
  if f64.const 0 local.set $from else local.get $from f64.trunc local.set $from end
  local.get $from f64.const 0 f64.lt
  if local.get $length f64.convert_i32_u local.get $from f64.add local.set $from end
  local.get $from local.get $length f64.convert_i32_u f64.ge if i32.const 0 return end
  local.get $from f64.const 0 f64.max i32.trunc_f64_u local.set $start
  local.get $needle struct.get $gc_value 0 i32.const 3 i32.eq
  local.get $needle struct.get $gc_value 1 local.get $needle struct.get $gc_value 1 f64.ne
  i32.and local.set $needle_nan
  local.get $array struct.get $gc_array 1 struct.get $gc_object 0 local.set $props
  block $done loop $scan
    local.get $slot local.get $props array.len i32.ge_u br_if $done
    local.get $props local.get $slot array.get $gc_props local.tee $entry
    struct.get $gc_entry 0 call $gc_key_index local.set $key
    local.get $key local.get $start i64.extend_i32_u i64.ge_u
    local.get $key local.get $length i64.extend_i32_u i64.lt_u i32.and
    if
      local.get $present i32.const 1 i32.add local.set $present
      local.get $entry struct.get $gc_entry 1 local.tee $item local.get $needle
      call $gc_equal if i32.const 1 return end
      local.get $needle_nan
      if
        local.get $item struct.get $gc_value 0 i32.const 3 i32.eq
        local.get $item struct.get $gc_value 1 local.get $item struct.get $gc_value 1 f64.ne
        i32.and if i32.const 1 return end
      end
    end
    local.get $slot i32.const 1 i32.add local.set $slot br $scan
  end end
  local.get $needle struct.get $gc_value 0 i32.eqz
  local.get $present local.get $length local.get $start i32.sub i32.lt_u i32.and)
