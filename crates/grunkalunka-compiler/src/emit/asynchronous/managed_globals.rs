//! One coroutine frame already persists in instance globals. Reference globals
//! are engine roots, never numeric spills; release them at terminal completion.
use std::fmt::Write;

pub(super) struct Globals {
    roots: Vec<(usize, &'static str)>,
}

impl Globals {
    pub fn new(lanes: &[&'static str]) -> Self {
        Self {
            roots: lanes
                .iter()
                .enumerate()
                .filter_map(|(index, lane)| {
                    lane.strip_prefix("(ref null ")
                        .and_then(|ty| ty.strip_suffix(')'))
                        .map(|ty| (index, ty))
                })
                .collect(),
        }
    }

    pub fn declare(&self, output: &mut String, lanes: &[&'static str]) {
        let mut roots = self.roots.iter().peekable();
        for (index, lane) in lanes.iter().enumerate() {
            if roots.peek().is_some_and(|(root, _)| *root == index) {
                let (_, ty) = roots.next().expect("ordered reference global");
                writeln!(output, "(global (mut {lane}) (ref.null {ty}))").unwrap();
            } else {
                writeln!(output, "(global (mut {lane}) ({lane}.const 0))").unwrap();
            }
        }
    }

    pub fn clear(&self, output: &mut String) {
        if !self.roots.is_empty() {
            output.push_str("call $gc_async_clear_roots\n");
        }
    }

    pub fn finish(&self, output: &mut String) {
        if self.roots.is_empty() {
            return;
        }
        output.push_str("(func $gc_async_clear_roots\n");
        for (index, ty) in &self.roots {
            writeln!(output, "ref.null {ty}\nglobal.set {index}").unwrap();
        }
        output.push_str(")\n");
    }
}
