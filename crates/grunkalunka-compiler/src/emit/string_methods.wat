;; Known Strings retain exact UTF-16 code units, including unpaired surrogates.
(func $gc_string_key (param $value (ref null $gc_value)) (result (ref null $gc_key))
  local.get $value struct.get $gc_value 0 i32.const 5 i32.ne if unreachable end
  local.get $value struct.get $gc_value 2 ref.cast (ref $gc_key))

;; ToIntegerOrInfinity for already-admitted Number arguments.
(func $gc_string_integer (param $number f64) (result f64)
  local.get $number local.get $number f64.ne
  if (result f64) f64.const 0 else local.get $number f64.trunc end)

;; Clamp before converting to a Wasm32 code-unit offset; infinities cannot trap.
(func $gc_string_position (param $length f64) (param $number f64) (param $relative i32)
  (result i32)
  local.get $number call $gc_string_integer local.set $number
  local.get $relative local.get $number f64.const 0 f64.lt i32.and
  if local.get $length local.get $number f64.add local.set $number end
  local.get $number f64.const 0 f64.max local.get $length f64.min i32.trunc_f64_u)

(func $gc_string_range (param $value (ref null $gc_value))
  (param $start f64) (param $end f64) (param $substring i32)
  (result (ref null $gc_value))
  (local $source (ref null $gc_key)) (local $text (ref null $gc_key))
  (local $length f64) (local $first i32) (local $last i32) (local $count i32)
  local.get $value call $gc_string_key local.tee $source array.len
  f64.convert_i32_u local.set $length
  local.get $length local.get $start local.get $substring i32.eqz
  call $gc_string_position local.set $first
  local.get $length local.get $end local.get $substring i32.eqz
  call $gc_string_position local.set $last
  local.get $last local.get $first i32.lt_u
  if
    local.get $substring
    if
      local.get $first local.get $last local.set $first local.set $last
    else local.get $first local.set $last end
  end
  local.get $last local.get $first i32.sub local.tee $count
  array.new_default $gc_key local.set $text
  local.get $text i32.const 0 local.get $source local.get $first local.get $count
  array.copy $gc_key $gc_key
  i32.const 5 f64.const 0 local.get $text struct.new $gc_value)

;; A character is one code unit, not a Unicode scalar value. Bounds precede the
;; integer conversion, so negative and unbounded Number positions cannot trap.
(func $gc_string_char_code_at (param $value (ref null $gc_value)) (param $index f64)
  (result f64) (local $text (ref null $gc_key))
  local.get $value call $gc_string_key local.set $text
  local.get $index call $gc_string_integer local.set $index
  local.get $index f64.const 0 f64.ge
  local.get $index local.get $text array.len f64.convert_i32_u f64.lt i32.and
  if (result f64)
    local.get $text local.get $index i32.trunc_f64_u array.get_u $gc_key f64.convert_i32_u
  else f64.const nan end)

(func $gc_string_char_at (param $value (ref null $gc_value)) (param $index f64)
  (result (ref null $gc_value)) (local $unit f64)
  local.get $value local.get $index call $gc_string_char_code_at local.set $unit
  i32.const 5 f64.const 0
  local.get $unit local.get $unit f64.ne
  if (result (ref null $gc_key))
    i32.const 0 array.new_default $gc_key
  else
    local.get $unit i32.trunc_f64_u i32.const 1 array.new $gc_key
  end
  struct.new $gc_value)

;; Exact code-unit comparison; no allocation, coercion, or normalization.
;; Private modes: 0=startsWith, 1=includes, 2=endsWith.
(func $gc_string_search (param $value (ref null $gc_value))
  (param $needle (ref null $gc_value)) (param $position f64) (param $mode i32)
  (result i32)
  (local $source (ref null $gc_key)) (local $search (ref null $gc_key))
  (local $length i32) (local $count i32) (local $start i32)
  (local $last i32) (local $index i32)
  local.get $value call $gc_string_key local.tee $source array.len local.set $length
  local.get $needle call $gc_string_key local.tee $search array.len local.set $count
  local.get $length f64.convert_i32_u local.get $position i32.const 0
  call $gc_string_position local.set $start
  local.get $count i32.eqz if i32.const 1 return end
  local.get $mode i32.const 2 i32.eq
  if
    ;; An endsWith position bounds the suffix end, not its start. Check before
    ;; subtraction so a longer needle cannot wrap into a valid array offset.
    local.get $count local.get $start i32.gt_u if i32.const 0 return end
    local.get $start local.get $count i32.sub local.set $start
  end
  ;; Subtract after clamping, before adding offsets, to avoid Wasm32 overflow.
  local.get $count local.get $length local.get $start i32.sub i32.gt_u
  if i32.const 0 return end
  local.get $length local.get $count i32.sub local.set $last
  loop $candidate
    i32.const 0 local.set $index
    block $mismatch
      loop $unit
        local.get $source local.get $start local.get $index i32.add array.get_u $gc_key
        local.get $search local.get $index array.get_u $gc_key
        i32.ne br_if $mismatch
        local.get $index i32.const 1 i32.add local.tee $index
        local.get $count i32.lt_u br_if $unit
      end
      i32.const 1 return
    end
    local.get $mode i32.const 1 i32.ne
    local.get $start local.get $last i32.eq i32.or
    if i32.const 0 return end
    local.get $start i32.const 1 i32.add local.set $start
    br $candidate
  end
  unreachable)

;; ECMAScript WhiteSpace plus LineTerminator, not Unicode's broader
;; White_Space property. Every member fits one UTF-16 code unit; surrogates,
;; U+0085, U+180E and U+200B are retained. Unsigned range checks handle underflow.
(func $gc_string_whitespace (param $unit i32) (result i32)
  local.get $unit i32.const 0x0009 i32.sub i32.const 4 i32.le_u
  local.get $unit i32.const 0x0020 i32.eq i32.or
  local.get $unit i32.const 0x00a0 i32.eq i32.or
  local.get $unit i32.const 0x1680 i32.eq i32.or
  local.get $unit i32.const 0x2000 i32.sub i32.const 10 i32.le_u i32.or
  local.get $unit i32.const 0x2028 i32.sub i32.const 1 i32.le_u i32.or
  local.get $unit i32.const 0x202f i32.eq i32.or
  local.get $unit i32.const 0x205f i32.eq i32.or
  local.get $unit i32.const 0x3000 i32.eq i32.or
  local.get $unit i32.const 0xfeff i32.eq i32.or)

;; The private side mask is 1=start, 2=end, 3=both. Bound each scan before
;; reading a code unit, including empty and all-white-space Strings. The
;; retained interval is copied unchanged, preserving interior space and UTF-16.
(func $gc_string_trim (param $value (ref null $gc_value)) (param $sides i32)
  (result (ref null $gc_value))
  (local $source (ref null $gc_key)) (local $text (ref null $gc_key))
  (local $first i32) (local $last i32) (local $count i32)
  local.get $value call $gc_string_key local.tee $source array.len local.set $last
  local.get $sides i32.const 1 i32.and
  if
    block $start_done
      loop $start
        local.get $first local.get $last i32.ge_u br_if $start_done
        local.get $source local.get $first array.get_u $gc_key
        call $gc_string_whitespace i32.eqz br_if $start_done
        local.get $first i32.const 1 i32.add local.set $first
        br $start
      end
    end
  end
  local.get $sides i32.const 2 i32.and
  if
    block $end_done
      loop $end
        local.get $first local.get $last i32.ge_u br_if $end_done
        local.get $source local.get $last i32.const 1 i32.sub array.get_u $gc_key
        call $gc_string_whitespace i32.eqz br_if $end_done
        local.get $last i32.const 1 i32.sub local.set $last
        br $end
      end
    end
  end
  local.get $last local.get $first i32.sub local.tee $count
  array.new_default $gc_key local.set $text
  local.get $text i32.const 0 local.get $source local.get $first local.get $count
  array.copy $gc_key $gc_key
  i32.const 5 f64.const 0 local.get $text struct.new $gc_value)

;; Return an exact UTF-16 position or -1. Source admission owns String/Number
;; proofs. Only lastIndexOf maps an explicit NaN position to positive infinity.
(func $gc_string_index (param $value (ref null $gc_value))
  (param $needle (ref null $gc_value)) (param $position f64) (param $backwards i32)
  (result f64)
  (local $source (ref null $gc_key)) (local $search (ref null $gc_key))
  (local $length i32) (local $count i32) (local $start i32)
  (local $last i32) (local $index i32)
  local.get $value call $gc_string_key local.tee $source array.len local.set $length
  local.get $needle call $gc_string_key local.tee $search array.len local.set $count
  local.get $backwards
  if
    local.get $position local.get $position f64.ne
    if f64.const inf local.set $position end
  end
  local.get $length f64.convert_i32_u local.get $position i32.const 0
  call $gc_string_position local.set $start
  local.get $count i32.eqz
  if local.get $start f64.convert_i32_u return end
  ;; Validate before subtracting: all candidate starts then fit [0,length-count]
  ;; and every matching-loop read is at most length-1, without Wasm32 overflow.
  local.get $count local.get $length i32.gt_u if f64.const -1 return end
  local.get $length local.get $count i32.sub local.set $last
  local.get $start local.get $last i32.gt_u
  if
    local.get $backwards
    if local.get $last local.set $start else f64.const -1 return end
  end
  loop $candidate
    i32.const 0 local.set $index
    block $mismatch
      loop $unit
        local.get $source local.get $start local.get $index i32.add array.get_u $gc_key
        local.get $search local.get $index array.get_u $gc_key
        i32.ne br_if $mismatch
        local.get $index i32.const 1 i32.add local.tee $index
        local.get $count i32.lt_u br_if $unit
      end
      local.get $start f64.convert_i32_u return
    end
    local.get $backwards
    if
      local.get $start i32.eqz if f64.const -1 return end
      local.get $start i32.const 1 i32.sub local.set $start
    else
      local.get $start local.get $last i32.eq if f64.const -1 return end
      local.get $start i32.const 1 i32.add local.set $start
    end
    br $candidate
  end
  unreachable)
