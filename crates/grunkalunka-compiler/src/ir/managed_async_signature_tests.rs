use super::*;

#[test]
fn async_managed_inputs_preserve_result_count_and_callback_guards() {
    for ty in [Type::ManagedObject, Type::ManagedArray, Type::ManagedValue] {
        let mut module = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
        let id = module.closure_signatures.intern_async(vec![ty]);
        assert!(module.closure_signatures.is_callback_signature(id));
        assert!(module.closure_signatures.is_returned_function(id));
        verify_module(&module).unwrap();
        for bad in [
            Type::ManagedObject,
            Type::ManagedArray,
            Type::ManagedValue,
            Type::Number,
        ] {
            module.closure_signatures.higher[id - 1001].result = bad;
            assert!(!module.closure_signatures.is_callback_signature(id));
            assert!(!module.closure_signatures.is_returned_function(id));
            assert!(
                verify_module(&module)
                    .unwrap_err()
                    .contains("invalid closure signature")
            );
        }
        module.closure_signatures.higher[id - 1001].result = Type::General;
        for parameters in [
            vec![Type::Predicate],
            vec![ty; 1001],
            vec![Type::Closure(id)],
            vec![Type::Closure(usize::MAX)],
        ] {
            module.closure_signatures.higher[id - 1001].parameters = parameters;
            assert!(!module.closure_signatures.is_callback_signature(id));
            assert!(!module.closure_signatures.is_returned_function(id));
            assert!(
                verify_module(&module)
                    .unwrap_err()
                    .contains("invalid closure signature")
            );
        }
    }
}
