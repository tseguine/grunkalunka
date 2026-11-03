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
