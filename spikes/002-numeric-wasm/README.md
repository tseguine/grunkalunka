# 002 - Numeric lowering research

The demonstrated compiler, numeric runtime and CLI have moved into the maintained
root Cargo workspace at development version 0.0.1. There is no duplicate compiler
implementation in this directory.

- [Compiler contract and replay](../../docs/compiler.md)
- [Maintained regression corpus](../../tests/compatibility/corpus.mjs)
- [Dependency screening](../../docs/dependencies.md)
- [Milestones](../../docs/milestones.md)

Historical generated artifacts remain ignored under this directory's `target/`.
New reports are written to root `target/compatibility-runs/`. Git history retains
the original experiment and subsequent lowering units.
