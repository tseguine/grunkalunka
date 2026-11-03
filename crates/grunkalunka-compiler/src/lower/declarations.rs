//! Declaration admission, initializer requests and binding publication.
//!
//! Each declarator is selected only after its predecessor has completed. The
//! source scheduler retains source charging, constructor selection and
//! publication order for statements and for-loop headers.
use super::*;

pub(super) enum DeclarationInitializer<'n, 'a> {
    Complete,
    Value(DeclarationValue<'n, 'a>),
    CheckClosure {
        local: &'n VariableDeclarator<'a>,
        closure: closures::LocalClosure,
    },
}

pub(super) struct DeclarationValue<'n, 'a> {
    local: &'n VariableDeclarator<'a>,
    source: &'n Expression<'a>,
    ty: Type,
    mode: evaluate::Mode,
    mutable: bool,
}

// Every selected mode produces exactly one ValueId. Discard, the evaluator's
// only void-result mode, is never an initializer request.

/// A function initializer owns ordinary function-value admission until its
/// result is delivered or its diagnostic unwinds. Other value kinds own no
/// extra admission; all retain the exact publication site and mutability.
pub(super) struct ActiveDeclarationValue<'n, 'a>(DeclarationValue<'n, 'a>);

impl<'n, 'a> ActiveDeclarationValue<'n, 'a> {
    pub(super) fn request(&self) -> (&'n Expression<'a>, evaluate::Mode) {
        (self.0.source, self.0.mode)
    }
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn admit_declaration(
        &self,
        declaration: &VariableDeclaration<'_>,
    ) -> Result<(), String> {
        if !matches!(
            declaration.kind,
            VariableDeclarationKind::Const | VariableDeclarationKind::Let
        ) || declaration.declare
            || declaration.declarations.is_empty()
        {
            return Err(unsupported(
                declaration,
                "initialized const or let identifiers are required",
            ));
        }
        Ok(())
    }

    /// Charge and validate just this local before selecting its initializer.
    /// Const closure metadata may request a disposable definition check;
    /// ordinary values become a typed request without evaluating their source.
    pub(super) fn prepare_declaration_initializer<'n, 'a>(
        &mut self,
        local: &'n VariableDeclarator<'a>,
        mutable: bool,
        region: &mut Region,
    ) -> Result<DeclarationInitializer<'n, 'a>, String> {
        if self.source_charging && self.local_count >= 128 {
            return Err(unsupported(local, "at most 128 source locals are admitted"));
        }
        if self.source_charging {
            self.local_count += 1;
        }
        if local.definite {
            return Err(unsupported(
                local,
                "definite-assignment assertions are not admitted",
            ));
        }
        let BindingPattern::BindingIdentifier(identifier) = &local.id else {
            return Err(unsupported(local, "local destructuring is not admitted"));
        };
        let closure = if !mutable && let Some(initializer) = local.init.as_ref() {
            match self.closure_target(initializer, 0) {
                Some(closure) => Some(closure),
                None => self.factory_result(initializer, region, 0)?,
            }
        } else {
            None
        };
        if let Some(closure) = closure {
            return self.publish_declaration_closure(local, mutable, closure, region);
        }
        let value = |source, ty, mode| {
            Ok(DeclarationInitializer::Value(DeclarationValue {
                local,
                source,
                ty,
                mode,
                mutable,
            }))
        };
        if let Some(initializer) = local.init.as_ref()
            && let Some(arity) = self.function_arity(initializer, 0)
        {
            return value(
                initializer,
                Type::Closure(arity),
                evaluate::Mode::Function(arity),
            );
        }
        if mutable
            && identifier
                .symbol_id
                .get()
                .is_some_and(|symbol| self.promise_parameters.is_mixed(&symbol))
        {
            if let Some(initializer) = local.init.as_ref() {
                return value(initializer, Type::AsyncValue, evaluate::Mode::AsyncValue);
            }
            let input = self.value(Type::General);
            Self::push(
                region,
                local,
                Op::Nullish {
                    result: input,
                    value: ir::Nullish::Undefined,
                },
            );
            let result = self.value(Type::AsyncValue);
            Self::push(region, local, Op::ToAsyncValue { result, input });
            self.commit_local_value(local, region, Type::AsyncValue, result, mutable)?;
            return Ok(DeclarationInitializer::Complete);
        }
        if let Some(initializer) = local.init.as_ref() {
            if self.pending_promise_shape(initializer, 0) {
                return value(initializer, Type::Promise, evaluate::Mode::Pending);
            }
            if self.fulfilled_promise_shape(initializer, 0) {
                return value(initializer, Type::FulfilledPromise, evaluate::Mode::Promise);
            }
            let ty = if let Some(ty) = self.managed_source_type(initializer, 0) {
                ty
            } else if !mutable
                && self.promise_parameters.identities
                && self.has_promise_choice(initializer, 0)
            {
                Type::AsyncValue
            } else if !mutable && self.const_boolean_initializer(initializer, 0) {
                Type::Predicate
            } else if identifier
                .symbol_id
                .get()
                .is_some_and(|symbol| self.general_bindings.contains(&symbol))
                || !self.numeric_result_shape(initializer, 0)
            {
                Type::General
            } else {
                Type::Number
            };
            let mode = match ty {
                Type::ManagedArray => evaluate::Mode::Array,
                Type::ManagedObject => evaluate::Mode::Object,
                Type::ManagedValue => evaluate::Mode::Managed,
                Type::General => evaluate::Mode::General,
                Type::Predicate => evaluate::Mode::Predicate,
                Type::Number => evaluate::Mode::Number,
                Type::AsyncValue => evaluate::Mode::AsyncValue,
                Type::Closure(_) => unreachable!("function values use their own cursor"),
                Type::FulfilledPromise => unreachable!("Promise initializer handled above"),
                Type::SettledPromise | Type::Promise => {
                    return Err(unsupported(
                        initializer,
                        "settled Promise source values are not admitted",
                    ));
                }
            };
            value(initializer, ty, mode)
        } else if mutable {
            let result = self.value(Type::General);
            // An implicit undefined is intrinsic, independent of its spelling
            // or any binding named undefined in the surrounding source.
            Self::push(
                region,
                local,
                Op::Nullish {
                    result,
                    value: ir::Nullish::Undefined,
                },
            );
            self.commit_local_value(local, region, Type::General, result, mutable)?;
            Ok(DeclarationInitializer::Complete)
        } else {
            Err("numeric local must have an initializer".into())
        }
    }

    pub(super) fn begin_declaration_value<'n, 'a>(
        &mut self,
        value: DeclarationValue<'n, 'a>,
    ) -> Result<ActiveDeclarationValue<'n, 'a>, String> {
        if matches!(value.ty, Type::Closure(_)) {
            self.enter_function_value(value.source)?;
        }
        Ok(ActiveDeclarationValue(value))
    }

    pub(super) fn abort_declaration_value(&mut self, active: ActiveDeclarationValue<'_, '_>) {
        if matches!(active.0.ty, Type::Closure(_)) {
            self.leave_function_value();
        }
    }

    pub(super) fn finish_declaration_value(
        &mut self,
        active: ActiveDeclarationValue<'_, '_>,
        region: &mut Region,
        value: ValueId,
    ) -> Result<(), String> {
        let DeclarationValue {
            local, ty, mutable, ..
        } = active.0;
        if matches!(ty, Type::Closure(_)) {
            self.leave_function_value();
            self.commit_function_local(local, region, ty, value, mutable)
        } else {
            self.commit_local_value(local, region, ty, value, mutable)
        }
    }

    pub(super) fn publish_declaration_closure<'n, 'a>(
        &mut self,
        local: &'n VariableDeclarator<'a>,
        mutable: bool,
        closure: closures::LocalClosure,
        region: &mut Region,
    ) -> Result<DeclarationInitializer<'n, 'a>, String> {
        let BindingPattern::BindingIdentifier(identifier) = &local.id else {
            unreachable!("declaration local was checked before selection");
        };
        if self.managed_kinds.identity_bodies.contains(&closure.arrow)
            || identifier
                .symbol_id
                .get()
                .is_some_and(|s| self.managed_kinds.identity_symbols.contains(&s))
        {
            let initializer = local.init.as_ref().expect("closure initializer");
            let arity = self
                .arrow_signature(self.arrow(closure.arrow))
                .ok_or_else(|| {
                    unsupported(initializer, "function value requires a known signature")
                })?;
            let value = self.materialize_closure(closure, region, initializer, arity)?;
            self.commit_function_local(local, region, Type::Closure(arity), value, mutable)?;
            return Ok(DeclarationInitializer::Complete);
        }
        // An alias reuses an already validated source body and its cost.
        if closure.symbol.is_none() {
            if !closure.checked {
                return Ok(DeclarationInitializer::CheckClosure { local, closure });
            }
            self.capture_cells(&closure, region);
        }
        self.commit_declaration_closure(local, closure)?;
        Ok(DeclarationInitializer::Complete)
    }

    pub(super) fn finish_declaration_closure_check(
        &mut self,
        local: &VariableDeclarator<'_>,
        closure: closures::LocalClosure,
        region: &mut Region,
    ) -> Result<(), String> {
        debug_assert!(closure.checked && closure.symbol.is_none());
        self.capture_cells(&closure, region);
        self.commit_declaration_closure(local, closure)
    }

    fn commit_declaration_closure(
        &mut self,
        local: &VariableDeclarator<'_>,
        mut closure: closures::LocalClosure,
    ) -> Result<(), String> {
        let BindingPattern::BindingIdentifier(identifier) = &local.id else {
            unreachable!("declaration local was checked before selection");
        };
        closure.symbol = Some(
            identifier
                .symbol_id
                .get()
                .ok_or("missing closure binding")?,
        );
        self.closures.push(closure);
        Ok(())
    }

    fn commit_function_local(
        &mut self,
        local: &VariableDeclarator<'_>,
        region: &mut Region,
        ty: Type,
        value: ValueId,
        mutable: bool,
    ) -> Result<(), String> {
        let BindingPattern::BindingIdentifier(identifier) = &local.id else {
            unreachable!("declaration local was checked before evaluation");
        };
        let slot = SlotId(self.slots.len());
        self.slots.push(Slot {
            ty,
            owner: region.id,
            mutable,
            parameter: false,
            span: local.span(),
        });
        Self::push(region, local, Op::Initialize { slot, value });
        self.bindings.push(NumericBinding {
            symbol: identifier
                .symbol_id
                .get()
                .ok_or("missing function binding")?,
            slot,
            mutable,
        });
        Ok(())
    }
}
