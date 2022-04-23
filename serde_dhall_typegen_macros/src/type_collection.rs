use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use serde_dhall::SimpleType;
use std::collections;

use crate::appendlist::AppendList;
use crate::create_error;
use crate::named_type::ClosedNamedType;
use crate::named_type::IndexedIdent;
use crate::util::IteratorSingle;
use crate::AggregateType;
use crate::NamedType;

#[derive(Debug, Eq, PartialEq)]
pub enum IdentOrigin {
    Local,
    FromSerdeDhallTypegen,
}

#[derive(Debug)]
pub struct PrimitiveIdents {
    bool: Ident,
    natural: Ident,
    integer: Ident,
    double: Ident,
    text: Ident,
    optional: Ident,
    list: Ident,
}

impl PrimitiveIdents {
    pub fn new(span: Span) -> Self {
        Self {
            bool: Ident::new("bool", span),
            natural: Ident::new("u64", span),
            integer: Ident::new("i32", span),
            double: Ident::new("OrderedF64", span),
            text: Ident::new("String", span),
            optional: Ident::new("Option", span),
            list: Ident::new("Vec", span),
        }
    }
}

// A named type that (optionally) has associated anonymous types
#[derive(Debug)]
struct RootType<'a> {
    ty: AggregateType<'a>,
    anonymous_types: AppendList<AggregateType<'a>>,
}

impl<'a> RootType<'a> {
    pub fn new(ty: AggregateType<'a>) -> Self {
        Self {
            ty,
            anonymous_types: AppendList::new(),
        }
    }
}

pub trait CanCow<T>: Into<T> {
    fn get_ref(&self) -> &T;
}

impl<T: Into<T>> CanCow<T> for T
where
    for<'a> &'a T: Into<&'a T>,
{
    fn get_ref(&self) -> &T {
        self
    }
}

#[derive(Debug)]
pub struct TypeCollection<'a> {
    by_ident: collections::HashMap<Ident, RootType<'a>>,
    pending_by_ident: AppendList<(Ident, RootType<'a>)>,
    primitives: &'a PrimitiveIdents,
}

impl<'a> TypeCollection<'a> {
    pub fn new(primitives: &'a PrimitiveIdents) -> Self {
        Self {
            by_ident: collections::HashMap::new(),
            pending_by_ident: AppendList::new(),
            primitives,
        }
    }

    pub fn get_by_ident(&self, ident: &Ident) -> Option<NamedType<'a, '_>> {
        self.get_by_ident_internal(ident)
            .map(|(ident, rt)| NamedType::new_named(ident, rt.ty))
    }

    fn get_by_ident_internal(&self, ident: &Ident) -> Option<(&Ident, &RootType<'a>)> {
        match self.by_ident.get_key_value(ident) {
            existing @ Some(_) => existing,
            None => self
                .pending_by_ident
                .iter()
                .filter(|(existing_ident, _)| ident == existing_ident)
                .map(|(existing_ident, rt)| (existing_ident, rt))
                .single()
                .ok(),
        }
    }

    pub fn get_by_structure<'p>(
        &self,
        context: Option<&Ident>,
        ty: AggregateType<'p>,
    ) -> Option<ClosedNamedType<'a, '_, 'p>> {
        self.get_by_structure_internal(context, ty)
    }

    fn get_by_structure_internal<'p>(
        &self,
        context: Option<&Ident>,
        ty: AggregateType<'p>,
    ) -> Option<ClosedNamedType<'a, '_, 'p>> {
        self.iter_with_context_internal(context)
            .filter_map(|nt| {
                ty.is_form_of(nt.ty())
                    .ok()
                    .map(|parameter| ClosedNamedType::new(nt, parameter))
            })
            .single()
            .ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = NamedType<'a, '_>> {
        self.by_ident
            .iter()
            .chain(self.pending_by_ident.iter().map(|(ident, rt)| (ident, rt)))
            .map(|(ident, rt)| NamedType::new_named(ident, rt.ty))
    }

    pub fn iter_with_context(
        &self,
        context: Option<&Ident>,
    ) -> impl Iterator<Item = NamedType<'a, '_>> {
        self.iter_with_context_internal(context)
    }

    fn iter_with_context_internal(
        &self,
        context: Option<&Ident>,
    ) -> impl Iterator<Item = NamedType<'a, '_>> {
        context
            .map(|ctxt| self.iter_anonymous_types(ctxt))
            .into_iter()
            .flatten()
            .chain(self.iter())
    }

    pub fn iter_anonymous_types(&self, context: &Ident) -> impl Iterator<Item = NamedType<'a, '_>> {
        let (context, rt) = self
            .get_by_ident_internal(context)
            .unwrap_or_else(|| panic!("Type with ident '{}' not found", context));
        rt.anonymous_types
            .iter()
            .enumerate()
            .map(move |(index, ty)| NamedType::new_anonymous(context, index, *ty))
    }

    pub fn get_or_create_by_ident(
        &self,
        ident: impl CanCow<Ident>,
        at: AggregateType<'a>,
    ) -> Result<NamedType<'a, '_>, TokenStream> {
        if let Some(existing_at) = self.get_by_ident(ident.get_ref()) {
            if at.is_form_of(existing_at.ty()).is_ok() {
                Ok(existing_at)
            } else {
                Err(create_error(&format!(
                    "Named type '{}' does not match an existing type with the same name",
                    ident.get_ref()
                )))
            }
        } else {
            let index = self.pending_by_ident.len();
            self.pending_by_ident
                .push((ident.into(), RootType::new(at)));
            let (ident, rt) = &self.pending_by_ident[index];
            Ok(NamedType::new_named(ident, rt.ty))
        }
    }

    pub fn get_or_create_by_structure<'s>(
        &'s self,
        context: &'s Ident,
        at: AggregateType<'a>,
    ) -> Result<ClosedNamedType<'a, 's, 'a>, TokenStream> {
        if let Some(name) = at.info()?.name_override() {
            let nt = self.get_or_create_by_ident(Ident::new(name, Span::call_site()), at)?;
            // PERF is_equivalent is run twice here, once inside `get_or_create_ident` discarding the type_parameter,
            // and once here, to get the type parameter
            let type_parameter = at.is_form_of(nt.ty()).unwrap_or_else(|_| {
                panic!(
                    "is_equivalent should not return inconsistent results ('{:?}' vs. '{:?}')",
                    at,
                    nt.ty()
                )
            });
            Ok(ClosedNamedType::new(nt, type_parameter))
        } else if let Some(existing_nt) = self.get_by_structure(Some(context), at) {
            Ok(existing_nt)
        } else {
            Ok(ClosedNamedType::new(
                self.create_by_structure(context, at)?,
                None,
            ))
        }
    }

    fn create_by_structure<'s>(
        &'s self,
        context: &'s Ident,
        at: AggregateType<'a>,
    ) -> Result<NamedType<'a, 's>, TokenStream> {
        if let Some(existing_nt) = self.get_by_structure(Some(context), at) {
            Err(create_error(&format!(
                "Anonymous type '{:?}' has the same structure as existing type '{}'",
                at,
                existing_nt.ident()
            )))
        } else if let Some((_ident, rt)) = self.get_by_ident_internal(context) {
            let index = rt.anonymous_types.len();
            rt.anonymous_types.push(at);

            Ok(NamedType::new_anonymous(
                context,
                index,
                rt.anonymous_types[index],
            ))
        } else {
            Err(create_error(&format!(
                "Context type '{}' does not exist",
                context
            )))
        }
    }

    #[allow(dead_code)]
    pub fn condense(&mut self) {
        // This moves our types from the AppendList to a HashMap for better lookup
        // Unfortunately, shared refs are currently held for the entire type generation process,
        // so we don't ever have a chance to condense, and we have O(n) type lookup
        let mut pending = AppendList::new();
        std::mem::swap(&mut self.pending_by_ident, &mut pending);
        for (ident, rt) in pending.into_iter() {
            let result = self.by_ident.insert(ident, rt);
            assert!(result.is_none(), "Named type Ident collision");
        }
    }

    pub fn get_idents<'s>(
        &'s self,
        context_ident: &'s Ident,
        st: &'a SimpleType,
    ) -> Result<(IdentOrigin, Vec<IndexedIdent<'s>>), TokenStream> {
        fn aggregate_type<'a, 's>(
            tc: &'s TypeCollection<'a>,
            context_ident: &'s Ident,
            at: AggregateType<'a>,
            v: &mut Vec<IndexedIdent<'s>>,
            from: &mut IdentOrigin,
        ) -> Result<(), TokenStream> {
            let nt = tc.get_or_create_by_structure(context_ident, at)?;
            v.push(nt.ident());
            if let Some(parameter) = nt.type_parameter() {
                simple_type(tc, context_ident, parameter, v, from)
            } else {
                Ok(())
            }
        }

        fn simple_type<'s, 'a>(
            tc: &'s TypeCollection<'a>,
            context_ident: &'s Ident,
            st: &'a SimpleType,
            v: &mut Vec<IndexedIdent<'s>>,
            from: &mut IdentOrigin,
        ) -> Result<(), TokenStream> {
            match st {
                SimpleType::Bool => v.push(IndexedIdent::new_named(&tc.primitives.bool)),
                SimpleType::Natural => v.push(IndexedIdent::new_named(&tc.primitives.natural)),
                SimpleType::Integer => v.push(IndexedIdent::new_named(&tc.primitives.integer)),
                SimpleType::Double => {
                    *from = IdentOrigin::FromSerdeDhallTypegen;
                    v.push(IndexedIdent::new_named(&tc.primitives.double));
                }
                SimpleType::Text => v.push(IndexedIdent::new_named(&tc.primitives.text)),
                SimpleType::Optional(o) => {
                    v.push(IndexedIdent::new_named(&tc.primitives.optional));
                    simple_type(tc, context_ident, &**o, v, from)?
                }
                SimpleType::List(l) => {
                    v.push(IndexedIdent::new_named(&tc.primitives.list));
                    simple_type(tc, context_ident, &**l, v, from)?
                }
                SimpleType::Record(r) => {
                    aggregate_type(tc, context_ident, AggregateType::new_record(r), v, from)?
                }
                SimpleType::Union(u) => {
                    aggregate_type(tc, context_ident, AggregateType::new_union(u), v, from)?
                }
            }

            Ok(())
        }

        let mut v = Vec::new();
        let mut from = IdentOrigin::Local;
        simple_type(self, context_ident, st, &mut v, &mut from)?;

        Ok((from, v))
    }
}
