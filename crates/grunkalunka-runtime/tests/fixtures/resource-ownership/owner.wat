(component
  (import "grunkalunka:owned-test/cells" (instance $cells
    (export "cell" (type $cell (sub resource)))
    (export "[method]cell.read"
      (func (param "self" (borrow $cell)) (result f64)))
  ))
  (alias export $cells "cell" (type $cell))
  (core func $read (canon lower (func $cells "[method]cell.read")))
  (core func $drop-cell (canon resource.drop $cell))
  (core module $m
    (import "host" "read" (func $read (param i32) (result f64)))
    (import "host" "drop-cell" (func $drop-cell (param i32)))
    (global $saved (mut i32) (i32.const -1))
    (func (export "roundtrip") (param $cell i32) (result i32)
      local.get $cell
      global.set $saved
      local.get $cell)
    (func (export "consume") (param $cell i32)
      local.get $cell
      call $drop-cell)
    (func (export "peek") (param $cell i32) (result f64)
      (local $value f64)
      local.get $cell
      call $read
      local.set $value
      local.get $cell
      call $drop-cell
      local.get $value)
    ;; No resource argument: no fresh guest allocation before stale-index use.
    (func (export "replay") (result f64)
      global.get $saved
      call $read)
    (func (export "double-drop") (param $cell i32)
      local.get $cell
      call $drop-cell
      local.get $cell
      call $drop-cell)
  )
  (core instance $i (instantiate $m
    (with "host" (instance
      (export "read" (func $read))
      (export "drop-cell" (func $drop-cell))
    ))
  ))
  (func (export "roundtrip") (param "cell" (own $cell)) (result (own $cell))
    (canon lift (core func $i "roundtrip")))
  (func (export "consume") (param "cell" (own $cell))
    (canon lift (core func $i "consume")))
  (func (export "peek") (param "cell" (borrow $cell)) (result f64)
    (canon lift (core func $i "peek")))
  (func (export "replay") (result f64)
    (canon lift (core func $i "replay")))
  (func (export "double-drop") (param "cell" (own $cell))
    (canon lift (core func $i "double-drop")))
)
