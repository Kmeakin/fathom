//! Operational semantics of the data description language.

use std::collections::HashMap;
use std::sync::Arc;

use crate::core::{Constant, Elim, Globals, Head, Item, Term, Value};

/// Evaluate a term into a semantic value.
pub fn eval(
    globals: &Globals,
    items: &HashMap<&str, Item>,
    // TODO: locals: &Locals<Arc<Value>>,
    term: &Term,
) -> Arc<Value> {
    match term {
        Term::Global(range, name) => match globals.get(name) {
            None => Arc::new(Value::Error(range.clone())),
            Some((_, term)) => match term {
                Some(term) => eval(globals, items, term),
                None => Arc::new(Value::Neutral(
                    Head::Global(range.clone(), name.clone()),
                    Vec::new(),
                )),
            },
        },
        Term::Item(range, name) => match items.get(name.as_str()) {
            None => Arc::new(Value::Error(range.clone())),
            Some(Item::Alias(alias)) => eval(globals, items, &alias.term),
            Some(Item::Struct(_)) => Arc::new(Value::Neutral(
                Head::Item(range.clone(), name.clone()),
                Vec::new(),
            )),
        },
        Term::Ann(term, _) => eval(globals, items, term),
        Term::Universe(range, universe) => Arc::new(Value::Universe(range.clone(), *universe)),
        Term::FunctionType(param_type, body_type) => {
            let param_type = eval(globals, items, param_type);
            let body_type = eval(globals, items, body_type);

            Arc::new(Value::FunctionType(param_type, body_type))
        }
        Term::FunctionElim(head, argument) => match eval(globals, items, head).as_ref() {
            Value::Neutral(head, elims) => {
                let mut elims = elims.clone(); // FIXME: clone?
                elims.push(Elim::Function(term.range(), eval(globals, items, argument)));
                Arc::new(Value::Neutral(head.clone(), elims))
            }
            _ => Arc::new(Value::Error(term.range())),
        },
        Term::Constant(range, constant) => {
            Arc::new(Value::Constant(range.clone(), constant.clone()))
        }
        Term::BoolElim(range, head, if_true, if_false) => {
            match eval(globals, items, head).as_ref() {
                Value::Neutral(Head::Global(head_range, name), elims) if elims.is_empty() => {
                    match name.as_str() {
                        "true" => eval(globals, items, if_true),
                        "false" => eval(globals, items, if_false),
                        _ => {
                            let mut elims = elims.clone(); // FIXME: clone?
                            elims.push(Elim::Bool(
                                range.clone(),
                                if_true.clone(),
                                if_false.clone(),
                            ));
                            Arc::new(Value::Neutral(
                                Head::Global(head_range.clone(), name.clone()),
                                elims,
                            ))
                        }
                    }
                }
                Value::Neutral(head, elims) => {
                    let mut elims = elims.clone(); // FIXME: clone?
                    elims.push(Elim::Bool(range.clone(), if_true.clone(), if_false.clone()));
                    Arc::new(Value::Neutral(head.clone(), elims))
                }
                _ => Arc::new(Value::Neutral(
                    Head::Error(head.range()),
                    vec![Elim::Bool(range.clone(), if_true.clone(), if_false.clone())],
                )),
            }
        }
        Term::IntElim(range, head, branches, default) => {
            match eval(globals, items, head).as_ref() {
                Value::Constant(_, Constant::Int(value)) => match branches.get(&value) {
                    Some(term) => eval(globals, items, term),
                    None => eval(globals, items, default),
                },
                Value::Neutral(head, elims) => {
                    let mut elims = elims.clone(); // FIXME: clone?
                    elims.push(Elim::Int(range.clone(), branches.clone(), default.clone()));
                    Arc::new(Value::Neutral(head.clone(), elims))
                }
                _ => Arc::new(Value::Neutral(
                    Head::Error(head.range()),
                    vec![Elim::Int(range.clone(), branches.clone(), default.clone())],
                )),
            }
        }
        Term::Error(range) => Arc::new(Value::Error(range.clone())),
    }
}

/// Read a neutral term back into the term syntax.
fn read_back_neutral(head: &Head, elims: &[Elim]) -> Term {
    elims.iter().fold(
        match head {
            Head::Global(range, name) => Term::Global(range.clone(), name.clone()),
            Head::Item(range, name) => Term::Item(range.clone(), name.clone()),
            Head::Error(range) => Term::Error(range.clone()),
        },
        |head, elim| match elim {
            Elim::Function(_, argument) => {
                Term::FunctionElim(Arc::new(head), Arc::new(read_back(argument)))
            }
            Elim::Bool(range, if_true, if_false) => Term::BoolElim(
                range.clone(),
                Arc::new(head),
                if_true.clone(),
                if_false.clone(),
            ),
            Elim::Int(range, branches, default) => Term::IntElim(
                range.clone(),
                Arc::new(head),
                branches.clone(),
                default.clone(),
            ),
        },
    )
}

/// Read a value back into the term syntax.
pub fn read_back(value: &Value) -> Term {
    match value {
        Value::Neutral(head, elims) => read_back_neutral(head, elims),
        Value::Universe(range, universe) => Term::Universe(range.clone(), *universe),
        Value::FunctionType(param_ty, body_ty) => {
            Term::FunctionType(Arc::new(read_back(param_ty)), Arc::new(read_back(body_ty)))
        }
        Value::Constant(range, constant) => Term::Constant(range.clone(), constant.clone()),
        Value::Error(range) => Term::Error(range.clone()),
    }
}

/// Check that two values are equal.
pub fn equal(val1: &Value, val2: &Value) -> bool {
    match (val1, val2) {
        (Value::Neutral(head0, elims0), Value::Neutral(head1, elims1)) => {
            read_back_neutral(head0, elims0) == read_back_neutral(head1, elims1)
        }
        (Value::Universe(_, universe0), Value::Universe(_, universe1)) => universe0 == universe1,
        (Value::FunctionType(param_ty0, body_ty0), Value::FunctionType(param_ty1, body_ty1)) => {
            equal(param_ty1, param_ty0) && equal(body_ty0, body_ty1)
        }
        (Value::Constant(_, constant0), Value::Constant(_, constant1)) => constant0 == constant1,
        // Errors are always treated as equal
        (Value::Error(_), _) | (_, Value::Error(_)) => true,
        // Anything else is not equal!
        (_, _) => false,
    }
}
