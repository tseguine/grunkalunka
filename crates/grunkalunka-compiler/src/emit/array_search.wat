;; Own data entries can be stored out of numeric order. Scan the selected range
;; and retain the lowest/highest matching index. Only includes reads holes as
;; Undefined or treats NaN as equal. It uses any nonnegative result as true.
;; Modes: 0 = includes, 1 = indexOf, 2 = lastIndexOf.
(func $gc_array_search (param $array (ref null $gc_array)) (param $needle (ref null $gc_value))
  (param $from f64) (param $mode i32) (result i64)
  (local $length i32) (local $start i32) (local $end i32) (local $props (ref null $gc_props))
  (local $slot i32) (local $present i32) (local $key i64) (local $best i64)
  (local $entry (ref null $gc_entry)) (local $item (ref null $gc_value)) (local $needle_nan i32)
  local.get $array struct.get $gc_array 0 local.tee $length i32.eqz if i64.const -1 return end
  local.get $from local.get $from f64.ne
  if f64.const 0 local.set $from else local.get $from f64.trunc local.set $from end
  local.get $from f64.const 0 f64.lt
  if local.get $length f64.convert_i32_u local.get $from f64.add local.set $from end
  local.get $mode i32.const 2 i32.eq
  if
    local.get $from f64.const 0 f64.lt if i64.const -1 return end
    local.get $from local.get $length i32.const 1 i32.sub f64.convert_i32_u f64.min
    i32.trunc_f64_u i32.const 1 i32.add local.set $end
  else
    local.get $from local.get $length f64.convert_i32_u f64.ge if i64.const -1 return end
    local.get $from f64.const 0 f64.max i32.trunc_f64_u local.set $start
    local.get $length local.set $end
  end
  local.get $mode i32.eqz
  local.get $needle struct.get $gc_value 0 i32.const 3 i32.eq i32.and
  local.get $needle struct.get $gc_value 1 local.get $needle struct.get $gc_value 1 f64.ne
  i32.and local.set $needle_nan
  i64.const -1 local.set $best
  local.get $array struct.get $gc_array 1 struct.get $gc_object 0 local.set $props
  block $done loop $scan
    local.get $slot local.get $props array.len i32.ge_u br_if $done
    local.get $props local.get $slot array.get $gc_props local.tee $entry
    struct.get $gc_entry 0 call $gc_key_index local.set $key
    local.get $key local.get $start i64.extend_i32_u i64.ge_u
    local.get $key local.get $end i64.extend_i32_u i64.lt_u i32.and
    if
      local.get $present i32.const 1 i32.add local.set $present
      local.get $entry struct.get $gc_entry 1 local.tee $item local.get $needle call $gc_equal
      if
        local.get $mode i32.eqz if local.get $key return end
        local.get $best i64.const -1 i64.eq
        local.get $mode i32.const 2 i32.eq
        if (result i32) local.get $key local.get $best i64.gt_s
        else local.get $key local.get $best i64.lt_s end
        i32.or if local.get $key local.set $best end
      else
        local.get $needle_nan
        if
          local.get $item struct.get $gc_value 0 i32.const 3 i32.eq
          local.get $item struct.get $gc_value 1 local.get $item struct.get $gc_value 1 f64.ne
          i32.and if local.get $key return end
        end
      end
    end
    local.get $slot i32.const 1 i32.add local.set $slot br $scan
  end end
  local.get $mode i32.eqz local.get $needle struct.get $gc_value 0 i32.eqz i32.and
  local.get $present local.get $end local.get $start i32.sub i32.lt_u i32.and
  if i64.const 0 return end
  local.get $best)
