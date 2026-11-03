//! The production activation owns its Region across completion and diagnostics.
//! No restoration lease or compatibility interpreter participates in this test.
use super::*;

#[test]
fn owned_evaluator_preserves_root_storage_on_completion_and_diagnostic() {
    for (source, admitted) in [("1;", true), ("Math.noSuch(1);", false)] {
        let allocator = oxc_allocator::Allocator::default();
        let parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::mjs()).parse();
        assert!(!parsed.fatal_error && parsed.diagnostics.is_empty());
        let built = oxc_semantic::SemanticBuilder::new_compiler()
            .with_build_nodes(true)
            .build(&parsed.program);
        assert!(built.diagnostics.is_empty());
        let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
            panic!("expected expression statement");
        };
        let mut lowering = Lowering {
            return_type: Type::Number,
            semantic: &built.semantic,
            stored: &stored_functions::Fields::default(),
            functions: &[],
            ui_import: None,
            general_bindings: &Default::default(),
            general_parameters: &Default::default(),
            managed_kinds: &Default::default(),
            number_calls: &Default::default(),
            general_results: &Default::default(),
            settled_results: &Default::default(),
            async_results: &Default::default(),
            callback_parameters: &Default::default(),
            promise_parameters: &Default::default(),
            bindings: Vec::new(),
            slots: Vec::new(),
            values: vec![Type::Number],
            next_region: 18,
            local_count: 0,
            statement_count: 0,
            in_loop_body: false,
            closures: Vec::new(),
            closure_expansions: 0,
            active_local_calls: 0,
            active_block_calls: 0,
            source_charging: true,
            inline_depth: 0,
            statement_depth: 0,
            allow_await: false,
            allow_async_closures: false,
            lifted: Vec::new(),
            lifted_base: 0,
            lifting_depth: 0,
            active_function_values: 0,
            active_function_results: 0,
            function_result_depth: 0,
            closure_signatures: &Default::default(),
        };
        let mut region = Region {
            id: RegionId(17),
            span: oxc_span::Span::new(2, 9),
            operations: Vec::with_capacity(8),
            exit: Exit::Yield(ValueId(0)),
        };
        region.operations.push(crate::ir::Operation {
            span: oxc_span::Span::new(2, 3),
            kind: Op::Constant {
                result: ValueId(0),
                value: 7.0,
            },
        });
        let original_storage = region.operations.as_ptr();
        let output = Lowering::take_cursor_region(&mut region);
        assert!(region.operations.is_empty());
        assert!(matches!(region.exit, Exit::Fallthrough));
        let mut cursor = EvaluationCursor::new(output, &statement.expression, 0, Mode::Number);

        match lowering.run_evaluation(&mut cursor, Mode::Number) {
            Ok(EvaluationRun::Complete(Some(value))) => {
                assert!(admitted, "{source}");
                assert_eq!(lowering.values[value.0], Type::Number);
            }
            Err(error) => {
                assert!(!admitted, "{source}: {error}");
                assert!(error.ends_with("expression is outside the numeric experiment"));
            }
            _ => panic!("scalar ownership probe must complete or reject"),
        }
        let owned = cursor.current_region();
        assert_eq!(owned.id, RegionId(17));
        assert_eq!(owned.span, oxc_span::Span::new(2, 9));
        assert_eq!(owned.operations.as_ptr(), original_storage);
        assert_eq!(owned.operations[0].span, oxc_span::Span::new(2, 3));
        assert!(matches!(
            owned.operations[0].kind,
            Op::Constant {
                result: ValueId(0),
                value: 7.0,
            }
        ));
        assert!(matches!(owned.exit, Exit::Yield(ValueId(0))));
        ACTIVE_EVALUATOR_RUNNERS.with(|active| assert_eq!(active.get(), 0));

        if admitted {
            region = cursor.into_region();
            assert_eq!(region.operations.as_ptr(), original_storage);
            assert_eq!(region.operations.len(), 2);
            assert!(matches!(region.exit, Exit::Yield(ValueId(0))));
        }
        // On a diagnostic the source owner discards the activation and its IR;
        // the caller placeholder is never mistaken for a completed Region.
    }
}
