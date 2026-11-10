//! Assign a saved iteration selection through its original source reference.
use super::*;

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn check_iteration_member(
        &self,
        source: IterationSource<'_, 'ast>,
        object: &Expression<'ast>,
        key: iteration::PropertyKey<'_, 'ast>,
    ) -> Result<(), String> {
        let Some(ty @ (Type::ManagedObject | Type::ManagedArray)) = self.receiver_type(object, 1)
        else {
            return Err(unsupported(
                source.left(),
                "iteration member targets require a proven object or array",
            ));
        };
        match key {
            iteration::PropertyKey::Literal(key) => {
                if ty == Type::ManagedArray && key.as_slice() == [108, 101, 110, 103, 116, 104] {
                    return Err(unsupported(
                        source.left(),
                        "iteration cannot assign unchecked array length",
                    ));
                }
                evaluate::managed_member_key(
                    source.left(),
                    key,
                    ty == Type::ManagedArray,
                    self.stored.dictionary(object.node_id()),
                )?;
            }
            iteration::PropertyKey::Runtime(key) => {
                if !self.accepts_runtime_destination_key(object, ty, key, 1) {
                    return Err(unsupported(
                        source.left(),
                        "iteration member runtime keys require an unsigned index or a proven own String key",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(super) fn resume_iteration_member_receiver<'n>(
        &mut self,
        frame: Box<ForOfFrame<'n, 'ast>>,
        body: Region,
        selected: ValueId,
        object: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        let (_, key) = iteration::property_iteration_target(frame.source.left())
            .expect("validated member head");
        match key {
            iteration::PropertyKey::Literal(key) => {
                self.finish_iteration_member(
                    frame,
                    body,
                    selected,
                    object,
                    ir::ManagedKey::Literal(key),
                    cursor,
                );
            }
            iteration::PropertyKey::Runtime(key) => {
                let uint32 = self.stored.uint32_keys.contains(key, 1);
                cursor.pending.push(Frame::ForOfMemberKey {
                    frame,
                    selected,
                    object,
                    uint32,
                });
                cursor.next = Some(Next::Expression {
                    source: key,
                    output: body,
                    depth: 1,
                    mode: if uint32 {
                        evaluate::Mode::Number
                    } else {
                        evaluate::Mode::Managed
                    },
                });
            }
        }
        Ok(())
    }

    pub(super) fn finish_iteration_member<'n>(
        &mut self,
        frame: Box<ForOfFrame<'n, 'ast>>,
        mut body: Region,
        selected: ValueId,
        object: ValueId,
        key: ir::ManagedKey,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) {
        Self::push(
            &mut body,
            frame.source.left(),
            match key {
                ir::ManagedKey::Literal(key) if self.values[object.0] == Type::ManagedArray => {
                    Op::ManagedArraySetOwn {
                        array: object,
                        key,
                        value: selected,
                    }
                }
                ir::ManagedKey::Literal(key) => Op::ManagedSetOwn {
                    object,
                    key,
                    value: selected,
                },
                key => Op::ManagedSetOwnDynamic {
                    object,
                    key,
                    value: selected,
                },
            },
        );
        self.begin_for_of_body(frame, body, cursor);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn iteration_member_targets_keep_receiver_key_and_descriptor_guards() {
        for (body, admitted) in [
            ("const o={x:a};for(o.x of [b]){}return o.x;", true),
            ("const o={x:''};for(o.x in {a:1}){}return o.x.length;", true),
            ("const xs=[];for(xs[a>>>0] of [b]){}return xs.length;", true),
            ("const o={x:a};for(o['x'] of [b]){}return o.x;", true),
            (
                "const d=Object.create(null);const k='x';for(d[k] of [b]){}return +(d[k]);",
                true,
            ),
            ("for(a.x of [b]){}return a;", false),
            ("const o=a?{x:b}:null;for(o.x of [b]){}return a;", false),
            ("const xs=[];for(xs.length of [b]){}return a;", false),
            (
                "const xs=[];const k='0';for(xs[k] of [b]){}return a;",
                false,
            ),
            (
                "const o={x:a};const k='x';for(o[k] of [b]){}return a;",
                true,
            ),
            (
                "const o={};const k=a?'x':'constructor';for(o[k] of [b]){}return a;",
                false,
            ),
            (
                "const o={x:'ok'};const k=a?'x':'y';for(o[k] of [b]){}return o.x.trim().length;",
                false,
            ),
            ("const xs=[];for(xs[a] of [b]){}return a;", false),
            ("const o={x:a};for(o.__proto__ of [b]){}return a;", false),
            (
                "const o={x:'safe'};const alias=o;for(alias.x of [b]){}return o.x.length;",
                false,
            ),
            (
                "const xs=['safe'];for(xs[a>>>0] of [b]){}return xs[0].length;",
                false,
            ),
            ("const o={x:a};for([o.x] of [[b]]){}return a;", false),
            ("const o={x:a};for(o.x of [b?{}:null]){}return o.x;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
        for source in [
            "export function run(a,b){for((a as object).x of [b]){}return a;}",
            "export function run(a,b){const xs=[];for(xs[a as number] of [b]){}return a;}",
        ] {
            assert!(crate::compile(source, true).is_err(), "{source}");
        }
    }

    #[test]
    fn iteration_member_key_depth_is_bounded_on_a_small_native_stack() {
        std::thread::Builder::new().stack_size(2*1024*1024).spawn(|| {
            for (wraps,admitted) in [(126,true),(127,false)] {
                let source=format!("export function run(a,b){{const xs=[];for(xs[{}a>>>0{}] of [b]){{}}return xs.length;}}","(".repeat(wraps),")".repeat(wraps));
                for typed in [false,true] {
                    let r=crate::compile(&source,typed);
                    assert_eq!(r.is_ok(),admitted,"{wraps}: {:?}",r.err());
                }
            }
        }).unwrap().join().unwrap();
    }
}
