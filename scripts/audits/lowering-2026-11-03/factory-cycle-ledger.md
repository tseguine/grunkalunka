# Factory/declaration call-cycle ledger

The checked question is limited to the result forwarded by
`resume_returned_factory_check` into `schedule_declaration_initializer`: can that
value select the scheduler's `Factory` arm? This is not a model of all compiler
paths and cannot establish whole-graph acyclicity or the stack bound of other SCCs.

Source snapshot: commit `16411448823b77252875f051796f005b0331fab2`, tree
`e8ef039e517b157f820b4694c3d0a99b1e628891`, clean `audit-composed` worktree.
Paths below are relative to `crates/grunkalunka-compiler/src/lower/`.

| ID | Kind | Source and modeled claim |
|---|---|---|
| S1 | Direct syntactic inventory | `control_cursor/factories.rs`: `start_declaration_factory_arguments` (110), `advance_factory_arguments` (193, 201), `finish_concise_factory_expression` (413); `control_cursor/factory_checks.rs`: `start_returned_factory_check` (183), `resume_returned_factory_check` (218); `control_cursor/initializers.rs`: `schedule_declaration_initializer` (118), `finish_declaration_factory` (183). These direct call sites form the seven-node syntactic SCC. |
| S2a–c | Direct successful returns | `declarations.rs`, `publish_declaration_closure` (302–340): identity branch returns `Materialize`; fresh unchecked anonymous closure branch returns `CheckClosure`; successful publication returns `Complete`. Error propagation exits through `Result::Err`. |
| S2d | Exhaustive, source-entailable normalization | Inspection of the whole producer body enumerates all successful return expressions. `Complete`, `Materialize`, and `CheckClosure` are the complete successful-output set. This is stronger than merely observing three examples and is represented explicitly. |
| S3 | Direct data flow | `control_cursor/factory_checks.rs`, `resume_returned_factory_check` (208–219): the Declaration branch binds `next` directly to `publish_declaration_closure(...)?`, then passes `next` unchanged into `schedule_declaration_initializer`. |
| S4 | Direct match dispatch | `control_cursor/initializers.rs`, `schedule_declaration_initializer` (65–130): only its `Factory(prepared)` arm directly calls `start_declaration_factory_arguments`; other enum variants enter their respective completion/value/check/materialization branches. |
| S5 | Definitional Rust enum semantics | `declarations.rs`, `DeclarationInitializer` (7–25): `Complete`, `Materialize`, `CheckClosure`, and `Factory` are distinct enum variants. Exact variant matching cannot equate their discriminants. |
| S6 | Direct queue operation | `control_cursor/initializers.rs`, `schedule_declaration_initializer` (66–75): `Complete` increments the declarator index, pushes `Frame::AdvanceDeclaration`, records `cursor.completed`, and returns. It does not directly restart the declaration loop. |
| D1 | Constructed, entailed graph semantics | Positive transitive closure of S1 describes syntactic reachability only. It ignores runtime data/branch constraints. |
| D2 | Constructed, entailed finite-state semantics | A forwarded producer result can select the direct factory arm only if its exact variant matches that arm. |
| D3 | Constructed, entailed exhaustive-set semantics | S2d and S5 explicitly exclude the `Factory` discriminant from this producer's success set. The model supplies opposing evidence for the proposed back edge from that exclusion. It does not use negation as failure. |

The producer's successful-output set is deliberately a safe overapproximation for
this particular caller. A returned closure already checked by the child machinery
may make `CheckClosure` unreachable here, but that stronger fact is unnecessary and
is not assumed. No claim is made that every possible producer branch occurs on an
admitted source input.

## Pre-evaluation review

All constants are quoted; every rule-head variable is bound in a positive body
literal. No negative body literal or closed-world rule is used. Syntactic reachability
and value-sensitive arm selection have separate predicates. The negated conclusion
depends on an explicit exhaustive return-set premise and enum-disjointness facts.
The model would report a contradiction if positive evidence for a Factory output
were later added while retaining the old exhaustive-set premise; it does not hide
that inconsistency.

The planned sensitivity check removes only S2d, retaining the known three return
expressions and all static call sites. Its purpose is to demonstrate that lacking
an exhaustive producer claim leaves Factory selection unknown, rather than making
absence of a positive Factory fact count as a proof of exclusion.

Syntax and evaluation receipts are saved separately. Their evidence counts are
properties of the gloss evaluator, not statistical confidence or empirical
probabilities about the compiler.

## Checked result

Both models passed syntax checking and evaluation with `paralogic-mk1` upstream
revision `ed2d50bff1de8fe759be3304b22a51aa442699bf`. The base model retains positive
syntactic self-reachability. For the bounded Factory-selection query it derives
zero supporting evidence atoms and six opposing atoms: exhaustive outputs,
unchanged forwarding, the Factory arm discriminant, and the three distinct-variant
facts. The `Complete` publication path positively derives the queued
`AdvanceDeclaration` action.

In the sensitivity model, removing only the exhaustive-output premise leaves the
Factory-selection query explicitly requested but absent from results. It is
unknown, not false. This confirms that the exclusion depends on the reviewed
producer body rather than treating a missing `Factory` fact as negative evidence.

Conditional on the recorded source correspondence, this particular static back
edge cannot select the `Factory` arm. The result does not eliminate other SCCs,
establish the behavior of unmodeled consumers, or give a native-byte stack bound.
