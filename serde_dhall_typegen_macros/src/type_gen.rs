use std::{borrow::Cow, fmt::Write};

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde_dhall::SimpleType;

use crate::{
    aggregate_type::{
        AggregateTypeInfo, AggregateTypeRecordFieldsIter, AggregateTypeUnionAlternativesIter,
    },
    create_error,
    named_type::IndexedIdent,
    type_collection::{IdentOrigin, PrimitiveIdents},
    AggregateType, NamedType, Options, TypeCollection,
};

#[derive(Debug)]
pub(crate) struct TypeGen<'a> {
    type_collection: TypeCollection<'a>,
    options: Options,
}

impl<'a> TypeGen<'a> {
    pub fn new(primitives: &'a PrimitiveIdents, options: Options) -> Self {
        Self {
            type_collection: TypeCollection::new(primitives),
            options,
        }
    }

    pub fn add_type(
        &mut self,
        ty: AggregateType<'a>,
        file_ident: Ident,
    ) -> Result<NamedType, TokenStream> {
        let ident = ty
            .info()?
            .name_override()
            .map(|s| Ident::new(s, Span::call_site()))
            .unwrap_or(file_ident);
        self.type_collection.get_or_create_by_ident(ident, ty)
    }

    fn get_generic_parameter(index: u32) -> Ident {
        // We only allow one type parameter for now, but support for multiple might be added some day
        let mut ident = Cow::Borrowed("T");
        if index > 0 {
            // T, T1, T2, T3, etc.
            ident
                .to_mut()
                .write_str(&index.to_string())
                .expect("String::write_str should not fail");
        }
        Ident::new(&ident, Span::call_site())
    }

    fn tokenize_field(
        &self,
        context_ident: &Ident,
        field_ident: Ident,
        field_type: &'a SimpleType,
    ) -> Result<TokenStream, TokenStream> {
        let field_type_tokens = self.tokenize_type_ident(context_ident, field_type)?;
        Ok(quote!(pub #field_ident: #field_type_tokens,))
    }

    fn tokenize_variant(
        &self,
        context_ident: &Ident,
        variant_ident: Ident,
        variant_type: Option<&'a SimpleType>,
    ) -> Result<TokenStream, TokenStream> {
        if let Some(variant_type) = variant_type {
            let variant_type_tokens = self.tokenize_type_ident(context_ident, variant_type)?;
            Ok(quote!(#variant_ident(#variant_type_tokens),))
        } else {
            Ok(quote!(#variant_ident,))
        }
    }

    fn tokenize_field_impl(
        &self,
        context_ident: &Ident,
        field_ident: Ident,
        field_type: &'a SimpleType,
    ) -> Result<TokenStream, TokenStream> {
        let get_func_str = field_ident.to_string().to_case(Case::Snake);
        let get_func_ident = Ident::new(&get_func_str, Span::call_site());
        let get_mut_func_ident = Ident::new(&format!("{}_mut", get_func_str), Span::call_site());

        let variant_type_tokens = self.tokenize_type_ident(context_ident, field_type)?;

        let get_func = quote!(
            pub fn #get_func_ident(&self) -> &#variant_type_tokens {
                &self.#field_ident
            }
        );
        let get_mut_func = quote!(
            pub fn #get_mut_func_ident(&mut self) -> &mut #variant_type_tokens {
                &mut self.#field_ident
            }
        );

        Ok(quote!(
            #get_func
            #get_mut_func
        ))
    }

    fn tokenize_variant_impl(
        &self,
        context_ident: &Ident,
        variant_ident: Ident,
        variant_type: Option<&'a SimpleType>,
    ) -> Result<TokenStream, TokenStream> {
        let get_func_str = variant_ident.to_string().to_case(Case::Snake);
        let get_func_ident = Ident::new(&get_func_str, Span::call_site());
        let is_func_ident = Ident::new(&format!("is_{}", get_func_str), Span::call_site());
        let get_mut_func_ident = Ident::new(&format!("{}_mut", get_func_str), Span::call_site());
        let into_func_ident = Ident::new(&format!("into_{}", get_func_str), Span::call_site());

        let mut is_func = TokenStream::new();
        let mut get_func = TokenStream::new();
        let mut get_mut_func = TokenStream::new();
        let mut into_func = TokenStream::new();

        if let Some(variant_type) = variant_type {
            let variant_type_tokens = self.tokenize_type_ident(context_ident, variant_type)?;

            get_func = quote!(
                pub fn #get_func_ident(&self) -> ::core::option::Option<&#variant_type_tokens> {
                    if let Self::#variant_ident(value) = self {
                        Some(value)
                    } else {
                        None
                    }
                }
            );
            get_mut_func = quote!(
                pub fn #get_mut_func_ident(&mut self) -> ::core::option::Option<&mut #variant_type_tokens> {
                    if let Self::#variant_ident(value) = self {
                        Some(value)
                    } else {
                        None
                    }
                }
            );
            into_func = quote!(
                pub fn #into_func_ident(self) -> ::core::result::Result<#variant_type_tokens, Self> {
                    if let Self::#variant_ident(value) = self {
                        Ok(value)
                    } else {
                        Err(self)
                    }
                }
            );
        } else {
            is_func = quote!(
                pub fn #is_func_ident(&self) -> bool {
                    match self {
                        Self::#variant_ident => true,
                        _ => false,
                    }
                }
            );
        }

        Ok(quote!(
            #is_func
            #get_func
            #get_mut_func
            #into_func
        ))
    }

    fn tokenize_type_ident(
        &self,
        context_ident: &Ident,
        field_type: &'a SimpleType,
    ) -> Result<TokenStream, TokenStream> {
        if let Ok(at) = AggregateType::try_new(field_type) {
            if at.is_generic_sentinel() {
                let ident = Self::get_generic_parameter(0);
                return Ok(quote!(#ident));
            }
            if at.is_unit() {
                return Ok(quote!(()));
            }
        }

        let (origin, v) = self.type_collection.get_idents(context_ident, field_type)?;
        let mut iter = v.into_iter().rev();

        let innermost = iter
            .next()
            .ok_or_else(|| create_error("No Idents returned"))?;
        let tokens = iter.fold(quote!(#innermost), |i, o| quote!(#o<#i>));
        Ok(match origin {
            IdentOrigin::FromSerdeDhallTypegen => quote!(::serde_dhall_typegen::#tokens),
            _ => tokens,
        })
    }

    fn tokenize_type(
        &self,
        context_ident: Option<&Ident>,
        rust_type: NamedType<'a, '_>,
    ) -> Result<TokenStream, TokenStream> {
        let info = rust_type.ty().info()?;
        //let ident = info.name_override().map(|s| Ident::new(s, Span::call_site())).unwrap_or(rust)
        assert!(context_ident.is_some() || !rust_type.ident().is_anonymous());
        let context_ident = context_ident.unwrap_or_else(|| rust_type.ident().base_ident());

        match info.iter_members()? {
            crate::aggregate_type::AggregateTypeMembers::Record(r) => {
                self.tokenize_struct(context_ident, rust_type.ident(), info, r)
            }
            crate::aggregate_type::AggregateTypeMembers::Union(u) => {
                self.tokenize_enum(context_ident, rust_type.ident(), info, u)
            }
        }
    }

    fn tokenize_struct(
        &self,
        context_ident: &Ident,
        ident: IndexedIdent<'_>,
        info: AggregateTypeInfo<'a>,
        r: AggregateTypeRecordFieldsIter<'a>,
    ) -> Result<TokenStream, TokenStream> {
        let mut tokens = TokenStream::new();

        for (field_name, field_type) in r.clone() {
            let field_tokens = self.tokenize_field(
                context_ident,
                Ident::new(field_name, Span::call_site()),
                field_type,
            )?;
            tokens.extend(field_tokens);
        }
        let generic_parameter = Self::get_generic_parameter(0);
        let generic = if info.is_open_generic() {
            quote!(<#generic_parameter>)
        } else {
            TokenStream::new()
        };
        let impl_tokens = self.tokenize_struct_impl(context_ident, ident, info, r)?;
        Ok(quote!(
        #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize, ::serde_dhall::StaticType)]
        pub struct #ident #generic {
            #tokens
        }
        #impl_tokens
        ))
    }

    fn tokenize_enum(
        &self,
        context_ident: &Ident,
        ident: IndexedIdent<'_>,
        info: AggregateTypeInfo<'a>,
        u: AggregateTypeUnionAlternativesIter<'a>,
    ) -> Result<TokenStream, TokenStream> {
        let mut tokens = TokenStream::new();

        for (variant_name, variant_type) in u.clone() {
            let variant_tokens = self.tokenize_variant(
                context_ident,
                Ident::new(variant_name, Span::call_site()),
                variant_type,
            )?;
            tokens.extend(variant_tokens);
        }

        let generic_parameter = Self::get_generic_parameter(0);
        let generic = if info.is_open_generic() {
            quote!(<#generic_parameter>)
        } else {
            TokenStream::new()
        };
        let impl_tokens = self.tokenize_enum_impl(context_ident, ident, info, u)?;

        Ok(quote!(
        #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize, ::serde_dhall::StaticType)]
        pub enum #ident #generic {
            #tokens
        }
        #impl_tokens
        ))
    }

    fn tokenize_struct_impl(
        &self,
        context_ident: &Ident,
        ident: IndexedIdent<'_>,
        info: AggregateTypeInfo<'a>,
        r: AggregateTypeRecordFieldsIter<'a>,
    ) -> Result<TokenStream, TokenStream> {
        let mut tokens = TokenStream::new();

        if self.options.struct_impl(ident.is_anonymous()).value {
            for (field_name, field_type) in r {
                let field_tokens = self.tokenize_field_impl(
                    context_ident,
                    Ident::new(field_name, Span::call_site()),
                    field_type,
                )?;
                tokens.extend(field_tokens);
            }

            if !tokens.is_empty() {
                let generic_parameter = Self::get_generic_parameter(0);
                let generic = if info.is_open_generic() {
                    quote!(<#generic_parameter>)
                } else {
                    TokenStream::new()
                };
                tokens = quote!(
                impl #generic #ident #generic {
                    #tokens
                })
            }
        }

        Ok(tokens)
    }

    fn tokenize_enum_impl(
        &self,
        context_ident: &Ident,
        ident: IndexedIdent<'_>,
        info: AggregateTypeInfo<'a>,
        u: AggregateTypeUnionAlternativesIter<'a>,
    ) -> Result<TokenStream, TokenStream> {
        let mut tokens = TokenStream::new();

        if self.options.enum_impl(ident.is_anonymous()).value {
            for (variant_name, variant_type) in u {
                let variant_tokens = self.tokenize_variant_impl(
                    context_ident,
                    Ident::new(variant_name, Span::call_site()),
                    variant_type,
                )?;
                let new_tokens = quote!(#variant_tokens);
                tokens.extend(new_tokens);
            }

            if !tokens.is_empty() {
                let generic_parameter = Self::get_generic_parameter(0);
                let generic = if info.is_open_generic() {
                    quote!(<#generic_parameter>)
                } else {
                    TokenStream::new()
                };
                tokens = quote!(
                impl #generic #ident #generic {
                    #tokens
                });
            }
        }

        Ok(tokens)
    }

    pub fn tokenize(self) -> Result<TokenStream, TokenStream> {
        fn tokenize_internal(tg: &TypeGen) -> Result<TokenStream, TokenStream> {
            let mut tokens = TokenStream::new();

            for nt in tg.type_collection.iter_with_context(None) {
                let new_tokens = tg.tokenize_type(None, nt)?;
                tokens.extend(new_tokens);

                if nt.ident().is_anonymous() {
                    panic!("Orphaned anonymous type '{}'", nt.ident().ident());
                }

                for anon_nt in tg
                    .type_collection
                    .iter_anonymous_types(nt.ident().base_ident())
                {
                    let new_tokens = tg.tokenize_type(Some(nt.ident().base_ident()), anon_nt)?;
                    tokens.extend(new_tokens);
                }
            }

            Ok(tokens)
        }

        tokenize_internal(&self)
    }
}
