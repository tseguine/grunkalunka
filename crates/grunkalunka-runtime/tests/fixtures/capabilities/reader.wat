(component
  (import "grunkalunka:r1/cells" (instance $cells
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
    ;; These probes deliberately violate the guest handle protocol.
    (func (export "forge") (result f64)
      i32.const 2147483647
      call $read)
    (func (export "retain") (param i32) (result f64)
      f64.const 0)
    (func (export "stash") (param $cell i32) (result f64)
      local.get $cell
      global.set $saved
      local.get $cell
      call $drop-cell
      f64.const 0)
    (func (export "replay") (result f64)
      global.get $saved
      call $read)
    (func (export "run")
      (param $cell i32) (param $a f64) (param $b f64) (result f64)
      (local $result f64)
      local.get $a
      local.get $b
      f64.add
      local.get $cell
      call $read
      f64.add
      local.set $result
      local.get $cell
      call $drop-cell
      local.get $result)
  )
  (core instance $i (instantiate $m
    (with "host" (instance
      (export "read" (func $read))
      (export "drop-cell" (func $drop-cell))
    ))
  ))
  (func (export "run")
    (param "cell" (borrow $cell))
    (param "a" f64) (param "b" f64) (result f64)
    (canon lift (core func $i "run")))
  (func (export "forge") (result f64)
    (canon lift (core func $i "forge")))
  (func (export "retain") (param "cell" (borrow $cell)) (result f64)
    (canon lift (core func $i "retain")))
  (func (export "stash") (param "cell" (borrow $cell)) (result f64)
    (canon lift (core func $i "stash")))
  (func (export "replay") (result f64)
    (canon lift (core func $i "replay")))
)
