(component
  (import "grunkalunka:documents/documents" (instance $documents
    (type $error (enum "denied" "too-large" "unknown-grant"))
    (export "read-error" (type $exported-error (eq $error)))
    (export "document" (type $document (sub resource)))
    (export "[method]document.read"
      (func (param "self" (borrow $document)) (result (result string (error $exported-error)))))
  ))
  (alias export $documents "read-error" (type $read-error))
  (alias export $documents "document" (type $document))
  ;; A private bounded allocation protocol: at most one UTF-8 result per call.
  ;; This is not a general allocator or a production guest runtime.
  (core module $storage
    (memory (export "memory") 1 1)
    (global $allocated (mut i32) (i32.const 0))
    (global $cleanups (mut i32) (i32.const 0))
    (func (export "realloc")
      (param $old i32) (param $old-size i32) (param $align i32) (param $size i32) (result i32)
      local.get $old
      local.get $old-size
      i32.or
      local.get $align
      i32.const 1
      i32.ne
      i32.or
      local.get $size
      i32.const 4096
      i32.gt_u
      i32.or
      global.get $allocated
      i32.or
      if unreachable end
      i32.const 1
      global.set $allocated
      i32.const 1024)
    (func (export "cleanup") (param $result i32)
      local.get $result
      if unreachable end
      i32.const 0
      i32.const 0
      i32.const 16
      memory.fill
      i32.const 1024
      i32.const 0
      i32.const 4096
      memory.fill
      i32.const 0
      global.set $allocated
      global.get $cleanups
      i32.const 1
      i32.add
      global.set $cleanups)
    (func (export "cleanup-count") (result i32) global.get $cleanups)
  )
  (core instance $storage (instantiate $storage))
  (core func $read (canon lower (func $documents "[method]document.read")
    (memory (core memory $storage "memory"))
    (realloc (core func $storage "realloc"))))
  (core func $drop-document (canon resource.drop $document))
  (core module $adapter
    (import "host" "read" (func $read (param i32 i32)))
    (import "host" "drop-document" (func $drop-document (param i32)))
    (func (export "read") (param $document i32) (result i32)
      local.get $document
      i32.const 0
      call $read
      local.get $document
      call $drop-document
      i32.const 0)
  )
  (core instance $entry (instantiate $adapter
    (with "host" (instance
      (export "read" (func $read))
      (export "drop-document" (func $drop-document))
    ))
  ))
  (func (export "read") (param "document" (borrow $document))
    (result (result string (error $read-error)))
    (canon lift (core func $entry "read")
      (memory (core memory $storage "memory"))
      (post-return (core func $storage "cleanup"))
    ))
  (func (export "cleanup-count") (result u32)
    (canon lift (core func $storage "cleanup-count")))
)
