//! Pretty printing utilities

use pretty::termcolor::ColorSpec;
use pretty::{BoxDoc, Doc};
use std::rc::Rc;

mod concrete;
mod core;

/// An effectively 'infinite' line length for when we don't have an explicit
/// width provided for pretty printing.
///
/// `pretty.rs` seems to bug-out and break on every line when using
/// `usize::MAX`, so we'll just use a really big number instead...
pub const FALLBACK_WIDTH: usize = 1_000_000;

pub type StaticDoc = Doc<'static, ColorSpec, BoxDoc<'static, ColorSpec>>;

/// Convert a datatype to a pretty-printable document
pub trait ToDoc {
    fn to_doc(&self) -> StaticDoc;
}

impl<'a, T: ToDoc> ToDoc for &'a T {
    fn to_doc(&self) -> StaticDoc {
        (*self).to_doc()
    }
}

impl<T: ToDoc> ToDoc for Rc<T> {
    fn to_doc(&self) -> StaticDoc {
        (**self).to_doc()
    }
}
