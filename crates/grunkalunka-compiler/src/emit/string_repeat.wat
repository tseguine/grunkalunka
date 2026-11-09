;; String.prototype.repeat for a source-proven primitive String and Number count.
;; Keep raw UTF-16 units, including NUL and lone surrogates. The source
;; admission excludes user coercion and overridden prototype methods.
;; Invalid negative/infinite counts or excess allocation are currently traps
;; until catchable intrinsic RangeErrors are represented.
(func $gc_string_repeat (param $value (ref null $gc_value))
  (param $times f64) (result (ref null $gc_value))
  (local $source (ref null $gc_key)) (local $target (ref null $gc_key))
  (local $length i32) (local $size i32) (local $written i32) (local $chunk i32)
  (local $required f64)
  local.get $value struct.get $gc_value 0 i32.const 5 i32.ne
  if unreachable end
  local.get $value struct.get $gc_value 2 ref.cast (ref $gc_key)
  local.tee $source array.len local.set $length
  ;; ToIntegerOrInfinity: fractions truncate toward zero before the range
  ;; check, so negative fractions in (-1,0) become valid negative zero.
  local.get $times local.get $times f64.ne
  if f64.const 0 local.set $times end
  local.get $times f64.trunc local.set $times
  local.get $times f64.const 0 f64.lt
  local.get $times f64.const inf f64.eq i32.or
  if unreachable end
  local.get $length i32.eqz
  local.get $times f64.const 0 f64.eq i32.or
  if
    i32.const 0 array.new_default $gc_key local.set $target
  else
    ;; A finite count may be enormous: reject length overflow before the
    ;; Wasm32 cast or allocation. Every admitted product here is exact in f64.
    local.get $length f64.convert_i32_u local.get $times f64.mul
    local.tee $required f64.const 4294967295 f64.gt
    if unreachable end
    local.get $required i32.trunc_f64_u local.set $size
    local.get $size array.new_default $gc_key local.set $target
    local.get $target i32.const 0 local.get $source i32.const 0
    local.get $length array.copy $gc_key $gc_key
    local.get $length local.set $written
    ;; Double the populated prefix in-place. Source and destination ranges
    ;; never overlap; the final chunk may be shorter than the prefix.
    block $done loop $copy
      local.get $written local.get $size i32.ge_u br_if $done
      local.get $size local.get $written i32.sub local.set $chunk
      local.get $chunk local.get $written i32.gt_u
      if local.get $written local.set $chunk end
      local.get $target local.get $written local.get $target i32.const 0
      local.get $chunk array.copy $gc_key $gc_key
      local.get $written local.get $chunk i32.add local.set $written
      br $copy
    end end
  end
  i32.const 5 f64.const 0 local.get $target struct.new $gc_value)
