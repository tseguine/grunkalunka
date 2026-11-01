;; Cells retain their logical numeric lanes and a traced edge for references.
(type $gc_cell (struct (field (mut i32)) (field (mut f64)) (field (mut eqref))))
;; Environment entries share cells; call-argument entries own fresh cells.
(type $gc_frame (array (mut (ref null $gc_cell))))
;; Allocated once per closure creation; boxing/copies retain this identity.
(type $gc_function (struct (field i32) (field (ref null $gc_frame))))
