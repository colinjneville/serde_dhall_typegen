use std::path;

use proc_macro2::Span;

use crate::Spanned;

#[derive(Debug)]
pub(crate) struct Options {
    pub path: (path::PathBuf, Span),
    pub anonymous_enum_impl: Spanned<bool>,
    pub named_enum_impl: Spanned<bool>,
    pub anonymous_struct_impl: Spanned<bool>,
    pub named_struct_impl: Spanned<bool>,
}

impl Options {
    pub fn new(path: path::PathBuf, span: Span) -> Self {
        Self { 
            path: (path, span), 
            anonymous_enum_impl: Spanned::new(true),
            named_enum_impl: Spanned::new(false),
            anonymous_struct_impl: Spanned::new(false),
            named_struct_impl: Spanned::new(false),
        }
    }

    pub fn enum_impl(&self, is_anonymous: bool) -> &Spanned<bool> {
        if is_anonymous {
            &self.anonymous_enum_impl
        } else {
            &self.named_enum_impl
        }
    }

    pub fn struct_impl(&self, is_anonymous: bool) -> &Spanned<bool> {
        if is_anonymous {
            &self.anonymous_struct_impl
        } else {
            &self.named_struct_impl
        }
    }
}