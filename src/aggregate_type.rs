use std::{collections::{self, HashMap}};

use proc_macro2::{TokenStream, Ident};
use serde_dhall::SimpleType;

use crate::{META_ENV_POSTFIX, META_ENV_PREFIX, META_FIELD_POSTFIX, META_FIELD_PREFIX_CONTENT, META_FIELD_PREFIX_NAME, util::{create_error, IteratorSingle}};

/// A wrapper for `SimpleType::Union` and `SimpleType::Record` that hides the modifications made to the raw Dhall for supporting metadata (e.g. type names)
#[derive(Debug, Copy, Clone)]
pub struct AggregateType<'a>(AggregateTypeInternal<'a>);

impl<'a> AggregateType<'a> {
    pub fn new_record(r: &'a HashMap<String, SimpleType>) -> Self {
        Self(AggregateTypeInternal::Record(r))
    }

    pub fn new_union(u: &'a HashMap<String, Option<SimpleType>>) -> Self {
        Self(AggregateTypeInternal::Union(u))
    }

    pub fn try_new(st: &'a SimpleType) -> Result<Self, TokenStream> {
        AggregateTypeInternal::try_new(st).map(Self)
    }

    pub fn info(self) -> Result<AggregateTypeInfo<'a>, TokenStream> {
        self.0.info()
    }

    pub fn is_generic_sentinel(self) -> bool {
        self.0.is_generic_sentinel()
    }

    pub fn is_unit(self) -> bool {
        self.0.is_unit()
    }
    
    pub fn is_form_of(self, other: AggregateType) -> Result<Option<&'a SimpleType>, ()> {
        fn is_eq_all<'a>(a: AggregateType<'a>, b: AggregateType) -> Result<Option<&'a SimpleType>, ()> {
            let ai = a.0.info().map_err(|_| ())?;
            let bi = b.0.info().map_err(|_| ())?;

            if ai.name_override() == bi.name_override()
            && ai.kind() == bi.kind() {
                let mut parameter: Option<&'a SimpleType> = None;
                for (ak, av) in ai.iter_members().map_err(|_| ())? {
                    let bv = bi.get_member(ak)?;
                    let result = match (av, bv) {
                        // Union alternatives with associated types or record values
                        (Some(av), Some(bv)) => is_eq(av, bv),
                        // Union alternatives with no associated type
                        (None, None) =>  Ok(None),
                        // Union alternatives with only one having no associated type
                        _ => Err(()),
                    }?;

                    match (parameter, result) {
                        (Some(p), Some(r)) => is_eq(p, r).map(|_| ())?,
                        (None, Some(_)) => parameter = result,
                        _ => { }
                    }
                }

                Ok(parameter)
            } else {
                Err(())
            }
        }

        fn is_eq<'a>(a: &'a SimpleType, b: &SimpleType) -> Result<Option<&'a SimpleType>, ()> {
            if AggregateTypeInternal::try_new(b).map(AggregateTypeInternal::is_generic_sentinel).unwrap_or(false) {
                return Ok(Some(a));
            }

            match (a, b) {
                (SimpleType::Bool, SimpleType::Bool)
              | (SimpleType::Natural, SimpleType::Natural)
              | (SimpleType::Integer, SimpleType::Integer)
              | (SimpleType::Double, SimpleType::Double)
              | (SimpleType::Text, SimpleType::Text) => Ok(None),

                (SimpleType::Optional(a), SimpleType::Optional(b))
              | (SimpleType::List(a), SimpleType::List(b)) => is_eq(a, b),

                (SimpleType::Record(a), SimpleType::Record(b)) => is_eq_all(AggregateType(AggregateTypeInternal::Record(a)), AggregateType(AggregateTypeInternal::Record(b))),
                (SimpleType::Union(a), SimpleType::Union(b)) => is_eq_all(AggregateType(AggregateTypeInternal::Union(a)), AggregateType(AggregateTypeInternal::Union(b))),
                _ => Err(())
            }
        }

        is_eq_all(self, other)
    }
}

#[derive(Debug, Copy, Clone)]
enum AggregateTypeInternal<'a> {
    Record(&'a collections::HashMap<String, SimpleType>),
    Union(&'a collections::HashMap<String, Option<SimpleType>>),
}

impl<'a> AggregateTypeInternal<'a> {
    pub fn try_new(st: &'a SimpleType) -> Result<Self, TokenStream> {
        match st {
            SimpleType::Record(r) => Ok(Self::Record(r)),
            SimpleType::Union(u) => Ok(Self::Union(u)),
            _ => Err(create_error("SimpleType is not an aggregate type"))
        }
    }

    pub fn info(self) -> Result<AggregateTypeInfo<'a>, TokenStream> {
        self.info_internal().map_err(|(env_name, e)| {
            let error_msg = match e {
                MetaError::IdentNotUnion => format!("The type passed to env:{} must be a single alternative union", env_name),
                MetaError::IdentNotSingleAlternative => format!("The union passed to env:{} must have a single alternative", env_name),
                MetaError::NotAggregateType => format!("The type marked by env:{} must be a record or a union", env_name),
                MetaError::InvalidOverrideIdent => format!("The name specified for env:{} is not a valid Rust identifier", env_name),
                MetaError::InvalidForm => format!("env:{} is used incorrectly, or there are conflicting members", env_name),
            };
            
            create_error(&error_msg)
        })
    }

    fn info_internal(self) -> Result<AggregateTypeInfo<'a>, (&'static str, MetaError)> {
        // This isn't perfectly robust, some abomination of nested meta stuff could confuse us, but it would have to be intentional
        match self {
            Self::Record(r) => {
                // Check for the the prefix function ($env:rust_type <MyName> ? (\(t: Type)->t) { ... }) fields
                let result = match (r.get(META_FIELD_PREFIX_NAME), r.get(META_FIELD_PREFIX_CONTENT)) {
                    (Some(name), Some(content)) => {
                        let content = Self::try_new(content).map_err(|_| (META_ENV_PREFIX, MetaError::NotAggregateType))?;
                        Some((name, content, META_ENV_PREFIX))
                    }
                    // No fields, prefix function was not used
                    (None, None) => None,
                    // Only one field found, user must be doing something weird
                    _ => return Err((META_ENV_PREFIX, MetaError::InvalidForm)),
                }
                .or_else(|| r.get(META_FIELD_POSTFIX).map(|ty_name_union| (ty_name_union, self, META_ENV_POSTFIX)));
 
                if let Some((name_ty, ty, env_name)) = result {
                    if let SimpleType::Union(u) = name_ty {
                        match u.iter().single() {
                            Ok((variant_name, _variant_type)) => 
                                // Ensure the name override will be a valid identifier
                                if syn::parse_str::<Ident>(variant_name.as_str()).is_ok() {
                                    Ok(AggregateTypeInfo::new(Some(variant_name.as_str()), ty))
                                } else {
                                    Err((env_name, MetaError::InvalidOverrideIdent))
                                }
                            Err(_) => Err((env_name, MetaError::IdentNotSingleAlternative))
                        }
                    } else {
                        Err((env_name, MetaError::IdentNotUnion))
                    }
                } else {
                    Ok(AggregateTypeInfo::new(None, self))
                }
            }
            // If a union is given an explicit name, it will be wrapped in a record, so if we see a union here, it does not have an explicit name
            Self::Union(_) => Ok(AggregateTypeInfo::new(None, self)),
        }
    }

    pub fn is_unit(self) -> bool {
        match self {
            Self::Record(r) => r.is_empty(),
            Self::Union(_) => false,
        }
    }

    pub fn is_generic_sentinel(self) -> bool {
        match self {
            Self::Record(_) => false,
            Self::Union(u) => u.iter().single() == Ok((&"__sentinel".to_string(), &None)),
        }
    }
}

/// Computed information from an `AggregateType`
#[derive(Debug, Clone, Copy)]
pub struct AggregateTypeInfo<'a> {
    name_override: Option<&'a str>,
    inner_aggregate: AggregateTypeInternal<'a>,
}

impl<'a> AggregateTypeInfo<'a> {
    fn new(name_override: Option<&'a str>, inner_aggregate: AggregateTypeInternal<'a>) -> Self {
        Self {
            name_override,
            inner_aggregate,
        }
    }

    pub fn name_override(&self) -> Option<&'a str> {
        self.name_override
    }

    pub fn kind(&self) -> AggregateKind {
        match &self.inner_aggregate {
            AggregateTypeInternal::Record(_) => AggregateKind::Record,
            AggregateTypeInternal::Union(_) => AggregateKind::Union,
        }
    }

    pub fn iter_members(&self) -> Result<AggregateTypeMembers<'a>, TokenStream> {
        Ok(match self.inner_aggregate {
            AggregateTypeInternal::Record(r) => AggregateTypeMembers::Record(AggregateTypeRecordFieldsIter(r.iter())),
            AggregateTypeInternal::Union(u) => AggregateTypeMembers::Union(AggregateTypeUnionAlternativesIter(u.iter())),
        })
    }

    pub fn get_member(&self, member_name: &str) -> Result<Option<&'a SimpleType>, ()> {
        match self.inner_aggregate {
            AggregateTypeInternal::Record(r) => r.get(member_name).map(Option::Some).ok_or(()),
            AggregateTypeInternal::Union(u) => u.get(member_name).map(Option::as_ref).ok_or(()),
        }
    }

    pub fn is_open_generic(&self) -> bool {
        if self.inner_aggregate.is_generic_sentinel() {
            true
        } else {
            match self.iter_members() {
                Ok(members) => members.into_iter()
                    .filter_map(|(_, st)| st)
                    .filter_map(|st| AggregateTypeInternal::try_new(st).ok())
                    .filter_map(|at| at.info().ok())
                    .any(|ati| ati.is_open_generic()),
                Err(_) => false,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateKind {
    Record,
    Union,
}

#[derive(Debug, Clone)]
pub struct AggregateTypeUnionAlternativesIter<'a>(collections::hash_map::Iter<'a, String, Option<SimpleType>>);
impl<'a> Iterator for AggregateTypeUnionAlternativesIter<'a> {
    type Item = (&'a str, Option<&'a SimpleType>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (k.as_str(), v.as_ref()))
    }
}

#[derive(Debug, Clone)]
pub struct AggregateTypeRecordFieldsIter<'a>(collections::hash_map::Iter<'a, String, SimpleType>);
impl<'a> Iterator for AggregateTypeRecordFieldsIter<'a> {
    type Item = (&'a str, &'a SimpleType);

    fn next(&mut self) -> Option<Self::Item> {
        // Don't expose the META_FIELD_POSTFIX name override field
        match self.0.next().map(|(k, v)| (k.as_str(), v)) {
            Some((META_FIELD_POSTFIX, _)) => self.next(),
            next => next,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AggregateTypeMembers<'a> {
    Record(AggregateTypeRecordFieldsIter<'a>),
    Union(AggregateTypeUnionAlternativesIter<'a>),
}

impl<'a> Iterator for AggregateTypeMembers<'a> {
    type Item = (&'a str, Option<&'a SimpleType>);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            AggregateTypeMembers::Record(r) => r.next().map(|(k, v)| (k, Some(v))),
            AggregateTypeMembers::Union(u) => u.next(),
        }
    }
}

enum MetaError {
    IdentNotUnion,
    IdentNotSingleAlternative,
    NotAggregateType,
    InvalidOverrideIdent,
    InvalidForm,
}
