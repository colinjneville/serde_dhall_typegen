use core::fmt;
use std::borrow::Cow;
use std::fmt::Debug;

use proc_macro2::Ident;
use quote::ToTokens;
use serde_dhall::SimpleType;

use crate::AggregateType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexedIdent<'a> {
    ident: &'a Ident,
    index: Option<usize>,
}

impl<'a> IndexedIdent<'a> {
    pub fn new_named(ident: &'a Ident) -> Self {
        Self {
            ident,
            index: None,
        }
    }

    pub fn new_anonymous(ident: &'a Ident, index: usize) -> Self {
        Self {
            ident,
            index: Some(index),
        }
    }

    pub fn base_ident(&self) -> &'a Ident {
        self.ident
    }

    pub fn ident(&self) -> Cow<'a, Ident> {
        if let Some(index) = self.index {
            Cow::Owned(Ident::new(&format!("{}Anon{}", self.ident, index), self.ident.span()))
        } else {
            Cow::Borrowed(self.ident)
        }
    }

    pub fn is_anonymous(&self) -> bool {
        self.index.is_some()
    }
}

impl<'a> ToTokens for IndexedIdent<'a> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ident().as_ref().to_tokens(tokens)
    }
}

impl<'a> fmt::Display for IndexedIdent<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.ident().as_ref(), f)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NamedType<'a, 'i> {
    ident: IndexedIdent<'i>,
    aggregate_type: AggregateType<'a>,
}

impl<'a, 'i> NamedType<'a, 'i> {
    pub fn new(ident: IndexedIdent<'i>, aggregate_type: AggregateType<'a>) -> Self {
        Self {
            ident,
            aggregate_type,
        }
    }

    pub fn new_named(ident: &'i Ident, aggregate_type: AggregateType<'a>) -> Self {
        Self::new(IndexedIdent::new_named(ident), aggregate_type)
    }

    pub fn new_anonymous(ident: &'i Ident, index: usize, aggregate_type: AggregateType<'a>) -> Self {
        Self::new(IndexedIdent::new_anonymous(ident, index), aggregate_type)
    }

    pub fn ident(&self) -> IndexedIdent<'i> {
        self.ident
    }

    pub fn ty(&self) -> AggregateType<'a> {
        self.aggregate_type
    }  
}

pub struct ClosedNamedType<'a, 'i, 'p> {
    ty: NamedType<'a, 'i>,
    type_parameter: Option<&'p SimpleType>,
}

impl<'a, 'i, 'p> ClosedNamedType<'a, 'i, 'p> {
    pub fn new(ty: NamedType<'a, 'i>, type_parameter: Option<&'p SimpleType>) -> Self {
        Self {
            ty,
            type_parameter,
        }
    }

    pub fn ident(&self) -> IndexedIdent<'i> {
        self.ty.ident
    }

    #[allow(dead_code)]
    pub fn ty(&self) -> AggregateType<'a> {
        self.ty.aggregate_type
    }

    pub fn type_parameter(&self) -> Option<&'p SimpleType> {
        self.type_parameter
    }
}
