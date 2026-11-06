;; table[n]..table[n+1] holds consecutive names of exactly n UTF-16 units.
;; The first directory entry also gives the header size (max length + 2).
(func $gc_prototype_has (param $key (ref null $gc_key))
  (param $table (ref $gc_key)) (result i32)
  (local $length i32) (local $start i32) (local $end i32) (local $unit i32)
  local.get $key array.len local.tee $length i32.eqz if i32.const 0 return end
  local.get $length local.get $table i32.const 0 array.get_u $gc_key
  i32.const 2 i32.sub i32.gt_u if i32.const 0 return end
  local.get $table local.get $length array.get_u $gc_key local.set $start
  local.get $table local.get $length i32.const 1 i32.add array.get_u $gc_key local.set $end
  block $missing loop $candidate
    local.get $start local.get $end i32.ge_u br_if $missing
    i32.const 0 local.set $unit
    block $different loop $compare
      local.get $unit local.get $length i32.ge_u if i32.const 1 return end
      local.get $key local.get $unit array.get_u $gc_key
      local.get $table local.get $start local.get $unit i32.add array.get_u $gc_key
      i32.ne br_if $different
      local.get $unit i32.const 1 i32.add local.set $unit br $compare
    end end
    local.get $start local.get $length i32.add local.set $start br $candidate
  end end
  i32.const 0)
