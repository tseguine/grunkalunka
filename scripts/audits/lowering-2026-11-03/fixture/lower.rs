pub fn enter() { a(); let mut machine = Machine {}; machine.step(); macro_cycle(); }
fn a() { b(); }
fn b() { a(); }
struct Machine {}
impl Machine { fn step(&mut self) { with(self, |state| state.step()); } }
fn with(state: &mut Machine, callback: impl FnOnce(&mut Machine)) { callback(state); }
fn macro_cycle() { debug_assert!({ macro_cycle(); true }); }
fn fake() { let _ = "fake()"; /* fake(); */ }
#[cfg(test)] fn hidden() { hidden(); }

struct BoxedMachine {}
impl BoxedMachine {
    fn local_enum_cycle(&mut self) {
        enum Work { Continue { child: Box<BoxedMachine> } }
        let work = Work::Continue { child: Box::new(BoxedMachine {}) };
        match work { Work::Continue { mut child } => child.local_enum_cycle() }
    }
}
fn unknown_boxed_cycle(input: impl FnOnce() -> Box<BoxedMachine>) {
    let mut state = Box::new(input());
    state.local_enum_cycle();
}

mod nested {
    use super::*;
    pub fn imported_cycle() { through_import(); }
}
fn through_import() { nested::imported_cycle(); }

use alias_target::target as inherited_alias;
mod alias_target {
    pub fn target() { super::alias_child::child(); }
}
mod alias_child {
    use super::*;
    pub fn child() { inherited_alias(); }
}
