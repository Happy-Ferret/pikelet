use codespan::{ByteIndex, ByteSpan};
use nameless::{self, Embed, Name, Var};

use syntax::concrete;
use syntax::core;

/// The precedence of a term
///
/// This is used to reconstruct the parentheses needed to reconstruct a valid
/// syntax tree in the concrete syntax
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct Prec(i8);

impl Prec {
    /// This term will never be wrapped in parentheses
    pub const NO_WRAP: Prec = Prec(-1);
    /// Precedence corresponding to `Term` in the parser
    pub const ANN: Prec = Prec(0);
    /// Precedence corresponding to `LamTerm` in the parser
    pub const LAM: Prec = Prec(1);
    /// Precedence corresponding to `PiTerm` in the parser
    pub const PI: Prec = Prec(2);
    /// Precedence corresponding to `AppTerm` in the parser
    pub const APP: Prec = Prec(3);
    /// Precedence corresponding to `AtomicTerm` in the parser
    pub const ATOMIC: Prec = Prec(4);
}

fn parens_if(should_wrap: bool, inner: concrete::Term) -> concrete::Term {
    match should_wrap {
        false => inner,
        true => concrete::Term::Parens(ByteSpan::default(), Box::new(inner)),
    }
}

// TODO: Use this for name-avoidance
// const USED_NAMES: &[&str] = &[
//     // Keywords
//     "as",
//     "else",
//     "_",
//     "module",
//     "if",
//     "import",
//     "then",
//     "Type",
//     // Primitives
//     "true",
//     "false",
//     "Bool",
//     "String",
//     "Char",
//     "U8",
//     "U16",
//     "U32",
//     "U64",
//     "I8",
//     "I16",
//     "I32",
//     "I64",
//     "F32",
//     "F64",
// ];

/// Translate something to the corresponding concrete representation
pub trait ToConcrete<T> {
    fn to_concrete(&self) -> T {
        self.to_concrete_prec(Prec::NO_WRAP)
    }

    fn to_concrete_prec(&self, prec: Prec) -> T;
}

impl ToConcrete<(concrete::Declaration, concrete::Declaration)> for core::Definition {
    fn to_concrete_prec(&self, _: Prec) -> (concrete::Declaration, concrete::Declaration) {
        // build up the type claim
        let new_ann = concrete::Declaration::Claim {
            name: (ByteIndex::default(), self.name.clone()),
            ann: self.ann.to_concrete_prec(Prec::ANN),
        };

        // build up the concrete definition
        let new_definition = {
            // pull lambda arguments from the body into the definition
            let (params, body) = match self.term.to_concrete_prec(Prec::ANN) {
                concrete::Term::Lam(_, params, body) => (params, *body),
                body => (vec![], body),
            };

            concrete::Declaration::Definition {
                span: ByteSpan::default(),
                name: self.name.clone(),
                params,
                body,
                wheres: vec![],
            }
        };

        (new_ann, new_definition)
    }
}

impl ToConcrete<concrete::Module> for core::Module {
    fn to_concrete_prec(&self, _: Prec) -> concrete::Module {
        use std::iter;

        let declarations = self.definitions
            .iter()
            .flat_map(|definition| {
                let (new_ann, new_definition) = definition.to_concrete();
                iter::once(new_ann).chain(iter::once(new_definition))
            })
            .collect();

        concrete::Module::Valid {
            name: (ByteIndex::default(), self.name.clone()),
            declarations,
        }
    }
}

impl ToConcrete<concrete::Term> for core::Constant {
    fn to_concrete_prec(&self, _: Prec) -> concrete::Term {
        use syntax::concrete::{Literal, Term};
        use syntax::core::Constant;

        let span = ByteSpan::default();

        match *self {
            // FIXME: Draw these names from some environment?
            Constant::Bool(true) => Term::Var(span.start(), String::from("true")),
            Constant::Bool(false) => Term::Var(span.start(), String::from("false")),

            Constant::String(ref value) => Term::Literal(span, Literal::String(value.clone())),
            Constant::Char(value) => Term::Literal(span, Literal::Char(value)),

            Constant::U8(value) => Term::Literal(span, Literal::Int(value as u64)),
            Constant::U16(value) => Term::Literal(span, Literal::Int(value as u64)),
            Constant::U32(value) => Term::Literal(span, Literal::Int(value as u64)),
            Constant::U64(value) => Term::Literal(span, Literal::Int(value)),

            // FIXME: Underflow for negative numbers
            Constant::I8(value) => Term::Literal(span, Literal::Int(value as u64)),
            Constant::I16(value) => Term::Literal(span, Literal::Int(value as u64)),
            Constant::I32(value) => Term::Literal(span, Literal::Int(value as u64)),
            Constant::I64(value) => Term::Literal(span, Literal::Int(value as u64)),

            Constant::F32(value) => Term::Literal(span, Literal::Float(value as f64)),
            Constant::F64(value) => Term::Literal(span, Literal::Float(value)),

            // FIXME: Draw these names from some environment?
            Constant::BoolType => Term::Var(span.start(), String::from("Bool")),
            Constant::StringType => Term::Var(span.start(), String::from("String")),
            Constant::CharType => Term::Var(span.start(), String::from("Char")),
            Constant::U8Type => Term::Var(span.start(), String::from("U8")),
            Constant::U16Type => Term::Var(span.start(), String::from("U16")),
            Constant::U32Type => Term::Var(span.start(), String::from("U32")),
            Constant::U64Type => Term::Var(span.start(), String::from("U64")),
            Constant::I8Type => Term::Var(span.start(), String::from("I8")),
            Constant::I16Type => Term::Var(span.start(), String::from("I16")),
            Constant::I32Type => Term::Var(span.start(), String::from("I32")),
            Constant::I64Type => Term::Var(span.start(), String::from("I64")),
            Constant::F32Type => Term::Var(span.start(), String::from("F32")),
            Constant::F64Type => Term::Var(span.start(), String::from("F64")),
        }
    }
}

impl ToConcrete<concrete::Term> for core::Term {
    fn to_concrete_prec(&self, prec: Prec) -> concrete::Term {
        match *self {
            core::Term::Ann(_, ref term, ref ty) => parens_if(
                Prec::ANN < prec,
                concrete::Term::Ann(
                    Box::new(term.to_concrete_prec(Prec::LAM)),
                    Box::new(ty.to_concrete_prec(Prec::ANN)),
                ),
            ),
            core::Term::Universe(_, level) => {
                let level = match level {
                    core::Level(0) => None,
                    core::Level(level) => Some(level),
                };

                parens_if(
                    Prec::APP < prec && level.is_some(),
                    concrete::Term::Universe(ByteSpan::default(), level),
                )
            },
            core::Term::Constant(_, ref c) => c.to_concrete(),
            core::Term::Var(_, Var::Free(Name::User(ref name))) => {
                concrete::Term::Var(ByteIndex::default(), name.to_string())
            },
            // core::Term::Var(_, Var::Free(Name::Gen(ref _name, ref _gen))) => {}
            core::Term::Var(_, Var::Free(ref name)) => {
                // TODO: use name if it is present, and not used in the current scope
                // TODO: otherwise create a pretty name
                concrete::Term::Var(ByteIndex::default(), name.to_string())
            },
            core::Term::Var(_, Var::Bound(_, _)) => {
                // TODO: Better message
                panic!("Tried to convert a term that was not locally closed");
            },
            core::Term::Pi(_, ref scope) => {
                let ((name, Embed(ann)), body) = nameless::unbind(scope.clone());

                let term = if body.free_vars().contains(&name) {
                    // TODO: use name if it is present, and not used in the current scope
                    // TODO: otherwise create a pretty name
                    // TODO: add the used name to the environment
                    // TODO: convert the body using the new environment

                    match body.to_concrete_prec(Prec::LAM) {
                        // TODO: check if the body can be collapsed to form a 'sugary' pi
                        // concrete::Term::Pi(_, params, body) => unimplemented!(),
                        body => concrete::Term::Pi(
                            ByteIndex::default(),
                            vec![(
                                vec![(ByteIndex::default(), name.to_string())],
                                ann.to_concrete_prec(Prec::APP),
                            )],
                            Box::new(body),
                        ),
                    }
                } else {
                    // The body is not dependent on the parameter - so let's use an arrow instead!
                    concrete::Term::Arrow(
                        Box::new(ann.to_concrete_prec(Prec::APP)),
                        Box::new(body.to_concrete_prec(Prec::LAM)),
                    )
                };

                parens_if(Prec::PI < prec, term)
            },
            core::Term::Lam(_, ref scope) => {
                let ((name, Embed(ann)), body) = nameless::unbind(scope.clone());
                // TODO: use name if it is present, and not used in the current scope
                // TODO: otherwise create a pretty name
                // TODO: add the used name to the environment

                // convert the body using the new environment
                let term = match body.to_concrete_prec(Prec::LAM) {
                    // TODO: check if the body can be collapsed to form a 'sugary' lambda
                    // concrete::Term::Lam(_, params, body) => unimplemented!(),
                    body => concrete::Term::Lam(
                        ByteIndex::default(),
                        vec![(
                            vec![(ByteIndex::default(), name.to_string())],
                            Some(Box::new(ann.to_concrete_prec(Prec::LAM))),
                        )],
                        Box::new(body),
                    ),
                };

                parens_if(Prec::LAM < prec, term)
            },
            core::Term::App(ref fn_term, ref arg) => parens_if(
                Prec::APP < prec,
                concrete::Term::App(
                    Box::new(fn_term.to_concrete_prec(Prec::NO_WRAP)),
                    vec![arg.to_concrete_prec(Prec::NO_WRAP)], // TODO
                ),
            ),
            core::Term::If(_, ref cond, ref if_true, ref if_false) => parens_if(
                Prec::LAM < prec,
                concrete::Term::If(
                    ByteIndex::default(),
                    Box::new(cond.to_concrete_prec(Prec::APP)),
                    Box::new(if_true.to_concrete_prec(Prec::APP)),
                    Box::new(if_false.to_concrete_prec(Prec::APP)),
                ),
            ),
            core::Term::RecordType(_, ref label, ref ann, ref rest) => {
                let mut fields = vec![];
                let mut label = label;
                let mut ann = ann;
                let mut rest = rest;

                loop {
                    fields.push((
                        ByteIndex::default(),
                        label.0.clone(),
                        Box::new(ann.to_concrete_prec(Prec::NO_WRAP)),
                    ));

                    match **rest {
                        core::Term::RecordType(_, ref next_label, ref next_ann, ref next_rest) => {
                            label = next_label;
                            ann = next_ann;
                            rest = next_rest;
                        },
                        core::Term::EmptyRecordType(_) => break,
                        _ => panic!("ill-formed record type"), // FIXME: better error
                    }
                }

                concrete::Term::RecordType(ByteSpan::default(), fields)
            },
            core::Term::Record(_, ref label, ref expr, ref rest) => {
                let mut fields = vec![];
                let mut label = label;
                let mut expr = expr;
                let mut rest = rest;

                loop {
                    fields.push((
                        ByteIndex::default(),
                        label.0.clone(),
                        Box::new(expr.to_concrete_prec(Prec::NO_WRAP)),
                    ));

                    match **rest {
                        core::Term::Record(_, ref next_label, ref next_expr, ref next_rest) => {
                            label = next_label;
                            expr = next_expr;
                            rest = next_rest;
                        },
                        core::Term::EmptyRecord(_) => break,
                        _ => panic!("ill-formed record"), // FIXME: better error
                    }
                }

                concrete::Term::Record(ByteSpan::default(), fields)
            },
            core::Term::EmptyRecordType(_) => {
                concrete::Term::RecordType(ByteSpan::default(), vec![])
            },
            core::Term::EmptyRecord(_) => concrete::Term::Record(ByteSpan::default(), vec![]),
            core::Term::Proj(_, ref expr, _, ref label) => concrete::Term::Proj(
                Box::new(expr.to_concrete_prec(Prec::ATOMIC)),
                ByteIndex::default(),
                label.0.clone(),
            ),
        }
    }
}

impl ToConcrete<concrete::Term> for core::Value {
    fn to_concrete_prec(&self, prec: Prec) -> concrete::Term {
        // FIXME: Make this more efficient?
        core::Term::from(self).to_concrete_prec(prec)
    }
}

impl ToConcrete<concrete::Term> for core::Neutral {
    fn to_concrete_prec(&self, prec: Prec) -> concrete::Term {
        // FIXME: Make this more efficient?
        core::Term::from(self).to_concrete_prec(prec)
    }
}
