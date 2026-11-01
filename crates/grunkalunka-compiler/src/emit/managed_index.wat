;; Accepted objects contain only enumerable data entries. Array length is a
;; separate non-enumerable field; copying the sparse property table excludes it.
(func $gc_spread_index (param $value i32) (result (ref null $gc_key))
  (local $digits i32) (local $remaining i32) (local $key (ref null $gc_key))
  i32.const 1 local.set $digits
  local.get $value local.set $remaining
  block $sized loop $size
    local.get $remaining i32.const 10 i32.lt_u br_if $sized
    local.get $remaining i32.const 10 i32.div_u local.set $remaining
    local.get $digits i32.const 1 i32.add local.set $digits br $size
  end end
  local.get $digits array.new_default $gc_key local.set $key
  loop $write
    local.get $digits i32.const 1 i32.sub local.set $digits
    local.get $key local.get $digits
    local.get $value i32.const 10 i32.rem_u i32.const 48 i32.add
    array.set $gc_key
    local.get $value i32.const 10 i32.div_u local.set $value
    local.get $digits br_if $write
  end
  local.get $key)
