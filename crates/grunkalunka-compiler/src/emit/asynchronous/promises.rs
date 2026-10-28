//! Guest-owned settlement records and FIFO reactions. Pending IR integration
//! follows; existing modules do not include this helper merely by importing it.
#[expect(dead_code, reason = "Pending Promise IR integration follows")]
pub(super) const RUNTIME: &str = include_str!("promises.wat");
