;; String own-key lookup uses the shared guest ToPropertyKey subset.
;; Kept separately so unrelated String methods do not need that helper.
(func $gc_string_get_key (param $value (ref null $gc_value)) (param $key (ref null $gc_value))
  (result (ref null $gc_value)) (local $name (ref null $gc_key))
  local.get $key call $gc_property_key local.tee $name call $gc_length_key
  if (result (ref null $gc_value))
    i32.const 3 local.get $value call $gc_string_key array.len f64.convert_i32_u
    ref.null eq struct.new $gc_value
  else
    local.get $value local.get $name call $gc_key_index f64.convert_i64_s call $gc_string_get_index
  end)
