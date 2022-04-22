mod aggregate_type;
mod appendlist;
mod named_type;
mod options;
mod schema;
mod type_collection;
mod type_gen;
mod util;

use std::collections::HashMap;
use std::ffi;
use std::path;
use std::path::Path;
use appendlist::AppendList;
use options::Options;
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::quote_spanned;
use serde_dhall::SimpleType;
use syn::Token;
use syn::parse_macro_input;
use proc_macro2::TokenStream;
use quote::quote;
use convert_case::{Case, Casing};

use aggregate_type::AggregateType;
use named_type::NamedType;
use schema::Schema;
use type_collection::PrimitiveIdents;
use type_collection::TypeCollection;
use type_gen::TypeGen;
use util::create_error;

#[derive(Debug, Default)]
struct Spanned<T> {
    pub value: T,
    pub _span: Option<Span>,
}

impl<T> Spanned<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            _span: None
        }
    }

    pub fn new_spanned(value: T, span: Span) -> Self {
        Self {
            value, 
            _span: Some(span),
        }
    }
}

#[allow(dead_code)]
struct AttributeOption {
    option: Ident,
    equals_token: Token![=],
    value: syn::Lit,
}

impl syn::parse::Parse for AttributeOption {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self { 
            option: input.parse()?, 
            equals_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

#[allow(dead_code)]
struct DhallTypesAttributeOptions {
    comma: Token![,],
    options: Option<syn::punctuated::Punctuated<AttributeOption, Token![,]>>,
}

impl syn::parse::Parse for DhallTypesAttributeOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            comma: input.parse()?,
            options: if input.is_empty() { None } else { Some(input.parse_terminated(AttributeOption::parse)?) },
        })
    }
}

struct DhallTypesAttribute {
    path: syn::LitStr,
    options: Option<DhallTypesAttributeOptions>,
}

impl DhallTypesAttribute {
    pub fn into_options(self) -> Result<Options, TokenStream> {
        let mut options = Options::new(self.path.value().into(), self.path.span());
        if let Some(DhallTypesAttributeOptions { options: Some(attribute_options), .. }) = self.options {
            for option in attribute_options.into_iter() {
                match option.option.to_string().as_str() {
                    "anonymous_enum_impl" => {
                        if let syn::Lit::Bool(value) = option.value {
                            options.anonymous_enum_impl = Spanned::new_spanned(value.value(), value.span());
                        } else {
                            return Err(util::create_spanned_error("Expected boolean literal", option.option.span()));
                        }
                    }
                    "named_enum_impl" => {
                        if let syn::Lit::Bool(value) = option.value {
                            options.named_enum_impl = Spanned::new_spanned(value.value(), value.span());
                        } else {
                            return Err(util::create_spanned_error("Expected boolean literal", option.option.span()));
                        }
                    }
                    "anonymous_struct_impl" => {
                        if let syn::Lit::Bool(value) = option.value {
                            options.anonymous_struct_impl = Spanned::new_spanned(value.value(), value.span());
                        } else {
                            return Err(util::create_spanned_error("Expected boolean literal", option.option.span()));
                        }
                    }
                    "named_struct_impl" => {
                        if let syn::Lit::Bool(value) = option.value {
                            options.named_struct_impl = Spanned::new_spanned(value.value(), value.span());
                        } else {
                            return Err(util::create_spanned_error("Expected boolean literal", option.option.span()));
                        }
                    }
                    "anonymous_impl" => {
                        if let syn::Lit::Bool(value) = option.value {
                            options.anonymous_enum_impl = Spanned::new_spanned(value.value(), value.span());
                            options.anonymous_struct_impl = Spanned::new_spanned(value.value(), value.span());
                        } else {
                            return Err(util::create_spanned_error("Expected boolean literal", option.option.span()));
                        }
                    }
                    "named_impl" => {
                        if let syn::Lit::Bool(value) = option.value {
                            options.named_enum_impl = Spanned::new_spanned(value.value(), value.span());
                            options.named_struct_impl = Spanned::new_spanned(value.value(), value.span());
                        } else {
                            return Err(util::create_spanned_error("Expected boolean literal", option.option.span()));
                        }
                    }
                    "enum_impl" => {
                        if let syn::Lit::Bool(value) = option.value {
                            options.anonymous_enum_impl = Spanned::new_spanned(value.value(), value.span());
                            options.named_enum_impl = Spanned::new_spanned(value.value(), value.span());
                        } else {
                            return Err(util::create_spanned_error("Expected boolean literal", option.option.span()));
                        }
                    }
                    "struct_impl" => {
                        if let syn::Lit::Bool(value) = option.value {
                            options.anonymous_struct_impl = Spanned::new_spanned(value.value(), value.span());
                            options.named_struct_impl = Spanned::new_spanned(value.value(), value.span());
                        } else {
                            return Err(util::create_spanned_error("Expected boolean literal", option.option.span()));
                        }
                    }
                    "impl" => {
                        if let syn::Lit::Bool(value) = option.value {
                            options.anonymous_enum_impl = Spanned::new_spanned(value.value(), value.span());
                            options.named_enum_impl = Spanned::new_spanned(value.value(), value.span());
                            options.anonymous_struct_impl = Spanned::new_spanned(value.value(), value.span());
                            options.named_struct_impl = Spanned::new_spanned(value.value(), value.span());
                        } else {
                            return Err(util::create_spanned_error("Expected boolean literal", option.option.span()));
                        }
                    }
                    _ => return Err(util::create_spanned_error(&format!("Unknown option '{}'", option.option), option.option.span())),
                }
            }
        }
        Ok(options)
    }
}

impl syn::parse::Parse for DhallTypesAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.parse()?,
            options: if input.is_empty() { None } else { Some(input.parse()?) },
        })
    }
}

pub(crate) const META_ENV_PREFIX: &str = "rust_type";
pub(crate) const META_FIELD_PREFIX_NAME: &str = "__rust_type_name";
pub(crate) const META_FIELD_PREFIX_CONTENT: &str = "__rust_type_content";
pub(crate) const META_ENV_POSTFIX: &str = "rust_struct";
pub(crate) const META_FIELD_POSTFIX: &str = "__rust_struct";

fn set_environment_variable() {
    std::env::set_var(META_ENV_PREFIX, format!("\\(Name: Type) -> \\(Content: Type) -> {{ {}: Name, {}: Content }}", META_FIELD_PREFIX_NAME, META_FIELD_PREFIX_CONTENT));
    std::env::set_var(META_ENV_POSTFIX, format!("\\(Name: Type) -> {{ {}: Name }}", META_FIELD_POSTFIX));
}

/// Create serialization-compatible Rust types in the following `mod` for the Dhall types contained in a given folder.
/// Hand-written `impl`s can be included in the `mod` block.
/// By default, the generated types are named as the Pascal case of the Dhall file they are defined in (e.g. `my_type.dhall` -> `MyType`), but this can be overwritten (see Dhall metadata).
/// Any sub-unions or records contained within these files will be assigned an arbitrary name by default (which may change between compiles).
///  
/// ### Example `my_dhall_stuff.rs`
/// ```ignore
///     #[serde_dhall_typegen::dhall_types("./dhall/schema/")]
///     mod dhall { }
/// ```
/// 
/// # Arguments
/// 
/// * A string literal path to a directory of .dhall file(s)
/// * Optional parameters in the form `name = literal`
///     * `impl = bool` - Should functions be generated for all types? Equivalent to setting `named_impl` and `anonymous_impl`
///     * `named_impl = bool` - Should functions be generated for named types? Equivalent to setting `named_enum_impl` and `named_struct_impl`
///     * `anonymous_impl = bool` - Should functions be generated for anonymous types? Equivalent to setting `anonymous_enum_impl` and `anonymous_struct_impl`
///     * `named_enum_impl = bool` - Should variant access functions be generated for anonymous enums? Defaults to `false`
///     * `named_struct_impl = bool` - Should member access functions be generated for anonymous structs? Defaults to `false`
///     * `anonymous_enum_impl = bool` - Should member access functions be generated for anonymous enums? Defaults to `true`
///     * `anonymous_struct_impl = bool` - Should member access functions be generated for anonymous structs? Defaults to `false`
/// 
/// # Dhall Input
/// 
/// Each .dhall file in the specified directory will be evaluated and must return a record, a union, a [schema](https://docs.dhall-lang.org/tutorials/Language-Tour.html#record-completion),
/// or a function taking a single `Type` parameter which returns one of the previous types.
/// 
/// # Generic Types
/// 
/// `dhall_types` supports generating types with a single type parameter when provided a Dhall file containing a function of type `Type -> Type`. 
/// The type parameter will be named `T`. Like non-generic types, member types will be interpreted as instances of generic types if possible. For example,  
/// 
/// ### my_generic.dhall
/// ``` dhall
///     \(T: Type) -> {
///       field: T
///     }
/// ```
/// 
/// ### my_type.dhall
/// ``` dhall
///     let MyGeneric = ./my_generic.dhall
///     {
///       a: MyGeneric Natural,
///       b: { field: Text },
///     }
/// ```
/// 
/// ... becomes...
/// 
/// ``` rust
///     pub struct MyGeneric<T> {
///       pub field: T,
///     }
///     pub struct MyType {
///       a: MyGeneric<u64>,
///       b: MyGeneric<String>,
///     }
/// ```
/// 
/// # Generated Functions
/// 
/// Anonymous enums have functions generated for each variant by default. If a variant `MyVariant` has an associated type `T`, the enum will have the following functions:
/// ```ignore
/// fn my_variant(&self) -> Option<&T>
/// fn my_variant_mut(&mut self) -> Option<&mut T>
/// fn into_my_variant(self) -> Result<T, Self>
/// ```
///
/// If the variant does not have an associated type, it will have the following function:
/// ```ignore
/// fn is_my_variant(&self) -> bool
/// ```
///
/// If struct functions are enabled, a field `my_field` of type `T` will have the following functions:
/// ```ignore
/// fn my_field(&self) -> &T
/// fn my_field_mut(&mut self) -> &mut T
/// ```
/// 
/// # Dhall Metadata
/// 
/// Because Dhall and Rust have fundamentally different type systems, it may be necessary to make some non-functional changes in the Dhall to generate Rust types as you intend.
/// Dhall's types are [structural](https://en.wikipedia.org/wiki/Structural_type_system), so two types with the same structure in two separate files are the same type as far as Dhall is concerned. This is considered an error when generating
/// Rust types, however, as it creates ambiguities when determining which Rust type should be used for a given Dhall type.  
/// To avoid the issue, you can assign 'name metadata' to your identical Dhall types so that they can be differentiated during evaluation. This will not affect your Dhall types in other
/// contexts - the resultant Dhall types are only modified when specific environment variables are set.  
/// The first method can be used with either records or unions and precedes the type:
/// 
/// ### file_a.dhall
/// ``` dhall
///     env:rust_type ? (\(T: Type)->\(T: Type)->T) <MyEnumA>
///     < A | B >
/// ```
/// 
/// ### file_b.dhall
/// ``` dhall
///     env:rust_type ? (\(T: Type)->\(T: Type)->T) <MyEnumB>
///     < A | B >
/// ```
/// 
/// ... generates...
/// 
/// ``` rust
///     pub enum MyEnumA {
///       A,
///       B,
///     }
///     pub enum MyEnumB {
///       A,
///       B,
///     }
/// ```
/// 
/// The second method is less verbose, but can only be used for records and comes after the type:
/// 
/// ``` dhall
///     {
///       name: Text
///     } 
///     //\\ (env:rust_type <MyStructA> ? {})
///
///     {
///       name: Text
///     } 
///     //\\ (env:rust_type <MyStructB> ? {})
/// ```
///
/// ... generates...
/// 
/// ``` rust
///     pub struct MyStructA { 
///       pub name: String,
///     }
///     pub struct MyStructB {
///       pub name: String,
///     }
/// ```
/// 
/// You can use `let` bindings to reduce the verbosity:
/// 
/// ``` dhall
///     let rust_type = env:rust_type ? (\(T: Type)->\(T: Type)->T)
///     let rust_struct = \(T: Type) -> (env:rust_struct T ? {})
///     let Abc = rust_type <Abc> < A | B | C >
///     let Def = rust_type <Def> < D | E | F >
///     let Ghi = { g: Bool, h: Bool, i: Bool } //\\ (rust_struct <Ghi>)
///     let Jkl = { j: Bool, k: Bool, l: Bool } //\\ (rust_struct <Jkl>)
/// ```
/// 
/// # Example
/// ## Before
///
/// ### ./dhall/schema/person.dhall
///
/// ``` dhall
///     {
///         name: Text,
///         age_range: <
///             Baby        |
///             Toddler     |
///             Child       |
///             Teenager    |
///             Adult       |
///             Senior
///         >,
///         occupation: Optional {
///             title: Text,
///             salary: Natural,
///         }
///     }
/// ```
///
/// ### ./src/dhall.rs
/// ```ignore
///     #[serde_dhall_typegen::dhall_types("./dhall/schema/")]
///     mod dhall { }
/// ```
///
/// ## After
///
/// ### ./src/dhall.rs (equivalent to)
/// ```rust
///     mod dhall {
///         #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize)]
///         pub struct Person {
///             pub age_range: PersonAnon0,
///             pub name: String,
///             pub occupation: Option<PersonAnon1>,
///         }
///    
///         #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize)]
///         pub enum PersonAnon0 {
///             Teenager,
///             Senior,
///             Baby,
///             Toddler,
///             Child,
///             Adult,
///         }
///    
///         impl PersonAnon0 {
///             pub fn is_teenager(&self) -> bool {
///                 match self {
///                     Self::Teenager => true,
///                     _ => false,
///                 }
///             }
///             pub fn is_senior(&self) -> bool {
///                 match self {
///                     Self::Senior => true,
///                     _ => false,
///                 }
///             }
///             pub fn is_baby(&self) -> bool {
///                 match self {
///                     Self::Baby => true,
///                     _ => false,
///                 }
///             }
///             pub fn is_toddler(&self) -> bool {
///                 match self {
///                     Self::Toddler => true,
///                     _ => false,
///                 }
///             }
///             pub fn is_child(&self) -> bool {
///                 match self {
///                     Self::Child => true,
///                     _ => false,
///                 }
///             }
///             pub fn is_adult(&self) -> bool {
///                 match self {
///                     Self::Adult => true,
///                     _ => false,
///                 }
///             }
///         }
///  
///         #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize)]
///         pub struct PersonAnon1 {
///             pub salary: u64,
///             pub title: String,
///         }
///     }
/// ```
#[proc_macro_attribute]
pub fn dhall_types(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let dhall_types_attribute = parse_macro_input!(attr as DhallTypesAttribute);
    let mut module = parse_macro_input!(item as syn::ItemMod);
    let span = module.mod_token.span;

    let ts = match dhall_types_attribute.into_options() {
        Ok(options) => {
            // Everything generated by dhall_types_internal should have the module's span, 
            // but avoid passing the span everywhere and just set it on the final TokenStream
            let (Ok(ts) | Err(ts)) = dhall_types_internal(options);
            quote_spanned!(span => #ts)
        }
        Err(e) => e,
    };
    
    if let Some((_brace, items)) = &mut module.content {
        items.push(syn::Item::Verbatim(ts));
        quote!(#module)
    } else {
        util::create_spanned_error("Attribute must be applied to a local module", span)
    }.into()
}

fn try_parse_as_schema(root_dhall_path_str: &str, dhall_path_str: &str) -> Option<Schema> {
    let str_type = format!("\
        let schema = {}
        in
        schema.Type",
        dhall_path_str
    );

    let str_default = format!("\
        let schema = {}
        in
        schema.default",
        dhall_path_str
    );

    let _cd = util::PushCd::new(Path::new(root_dhall_path_str)).unwrap();
    let deser_type = serde_dhall::from_str(str_type.as_str());
    let deser_default = serde_dhall::from_str(str_default.as_str());
    
    let schema_type = deser_type.parse().ok()?;
    let schema_default = deser_default.parse().ok()?;

    let schema = Schema { r#type: schema_type, default: schema_default };
    Some(schema)
}

fn try_parse_as_aggregate_type(root_dhall_path_str: &str, dhall_path_str: &str) -> Option<SimpleType> {
    let _cd = util::PushCd::new(Path::new(root_dhall_path_str)).unwrap();
    let deser = serde_dhall::from_file(dhall_path_str);
    deser.parse().ok()
}

fn new_generic_sentinel() -> SimpleType {
    SimpleType::Union(HashMap::from([("__sentinel".to_string(), None)]))
}

fn try_parse_as_open_type(root_dhall_path_str: &str, dhall_path_str: &str) -> Option<SimpleType> {
    let str_type = format!("\
        let func = {}
        in
        func {}",
        dhall_path_str,
        new_generic_sentinel()
    );
    let _cd = util::PushCd::new(Path::new(root_dhall_path_str)).unwrap();
    let deser = serde_dhall::from_str(&str_type);
    deser.parse().ok()
}

fn dhall_types_internal(options: Options) -> Result<TokenStream, TokenStream> {
    let dhall_path = path::Path::new(&options.path.0).to_path_buf();

    set_environment_variable();

    // Track directories we haven't iterated yet
    let mut directory_stack = vec![dhall_path.clone()];

    let ty_storage = AppendList::new();

    let primitives = PrimitiveIdents::new(Span::call_site());

    let mut typegen = TypeGen::new(&primitives, options);

    while !directory_stack.is_empty() {
        let current_path = directory_stack.pop().unwrap();
        for entry in current_path.read_dir().map_err(|e| create_error(&format!("Directory read error: {}", e)))? {
            let entry = entry.map_err(|e| create_error(&format!("File read error: {}", e)))?;
            let entry_metadata = entry.metadata().map_err(|e| create_error(&e.to_string()))?;
            if entry_metadata.is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext.to_ascii_lowercase() == ffi::OsStr::new("dhall") {
                        use path_slash::PathBufExt;

                        fn invalid_type_error(path: &str, err: Option<serde_dhall::Error>) -> Result<TokenStream, TokenStream> {
                            Err(create_error(&format!("Dhall type in file '{}' is not a Record, Union, or schema: {:?}", path, err)))
                        }

                        let type_str = entry.path().file_stem().ok_or_else(|| create_error("No file name"))?.to_str().ok_or_else(|| create_error("Invalid file name"))?.to_case(Case::Pascal);
                        
                        let relative_path = Path::new(".").join(pathdiff::diff_paths(entry.path(), &dhall_path).ok_or_else(|| create_error(&format!("Unable to create relative path for file '{}'", entry.path().display())))?);
                        
                        let relative_path_str = relative_path.to_slash().ok_or_else(|| create_error(&format!("Invalid unicode in file '{}'", entry.path().display())))?;

                        let root_dhall_path_str = dhall_path.as_os_str().to_string_lossy();
                        let ty = if let Some(schema) = try_parse_as_schema(&root_dhall_path_str, &relative_path_str) {
                            schema.r#type
                        } else if let Some(ty) = try_parse_as_aggregate_type(&root_dhall_path_str, &relative_path_str) {
                            ty
                        } else if let Some(ty) = try_parse_as_open_type(&root_dhall_path_str, &relative_path_str) {
                            ty
                        } else {
                            let _cd = util::PushCd::new(Path::new(root_dhall_path_str.as_ref())).unwrap();
                            let deser = serde_dhall::from_file(relative_path_str.as_str());
                            return invalid_type_error(&relative_path_str, deser.parse::<SimpleType>().err());
                        };

                        ty_storage.push(ty);
                        let last_ty_index = ty_storage.len() - 1;
                        let ty = AggregateType::try_new(&ty_storage[last_ty_index]).map_err(|_| create_error(&format!("'{}' does not contain a record, union, or a function returning one", relative_path_str)))?;

                        let ident = Ident::new(&type_str, Span::call_site());
                        typegen.add_type(ty, ident)?;
                    }
                }
            } else if entry_metadata.is_dir() {
                directory_stack.push(entry.path().to_path_buf());
            } else {
                // Skip sym-links, etc. for now
            }
        }
    }

    typegen.tokenize()
}
