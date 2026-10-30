//! Synthetic deep regions exercise the routing scan only; they are not admitted
//! sources or a proof that the sync/async emitters accept arbitrary nesting.
use super::*;
use crate::ir::{Operation, RegionId};
use oxc_span::Span;

fn nested(depth: usize, exit: Exit) -> Region {
    let span = Span::new(0, 0);
    let mut region = Region {
        id: RegionId(0),
        span,
        operations: Vec::new(),
        exit,
    };
    for _ in 0..depth {
        region = Region {
            id: RegionId(0),
            span,
            operations: vec![Operation {
                span,
                kind: Op::Block(region),
            }],
            exit: Exit::Fallthrough,
        };
    }
    region
}

fn drain(mut region: Region) {
    while let Some(operation) = region.operations.pop() {
        let Op::Block(child) = operation.kind else {
            panic!("expected synthetic block")
        };
        region = child;
    }
}

#[test]
fn async_routing_scan_avoids_native_recursion_for_deep_regions() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            const DEPTH: usize = 4096;
            let plain = nested(DEPTH, Exit::Fallthrough);
            assert!(!region_has_control(&plain, false));
            assert!(!region_has_control(&plain, true));
            drain(plain);

            let throwing = nested(DEPTH, Exit::Throw(ValueId(0)));
            assert!(region_has_control(&throwing, true));
            drain(throwing);
        })
        .expect("stack probe thread")
        .join()
        .expect("stack probe completed");
}
