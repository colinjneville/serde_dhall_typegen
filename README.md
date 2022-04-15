# serde_dhall_typegen
A macro for automatically generating Rust structs and enums from Dhall types. Dhall values of these types can then be deserialized into Rust with the `serde_dhall` crate.  
Currently customizability is limited and the exact Rust generated is very subject to change, so it is only recommended to use this crate for prototyping while your Dhall schemas are changing often.

# Usage
Create serialization-compatible Rust types in the following `mod` for the Dhall types contained in a given folder.
Hand-written `impl`s can be included in the `mod` block.
By default, the generated types are named as the Pascal case of the Dhall file they are defined in (e.g. `my_type.dhall` -> `MyType`), but this can be overwritten (see Dhall metadata).
Any sub-unions or records contained within these files will be assigned an arbitrary name by default (which may change between compiles).

# Arguments

* A string literal path to a directory of .dhall file(s)
* Optional parameters in the form `name = literal`
    * `impl = bool` - Should functions be generated for all types? Equivalent to setting `named_impl` and `anonymous_impl`
    * `named_impl = bool` - Should functions be generated for named types? Equivalent to setting `named_enum_impl` and `named_struct_impl`
    * `anonymous_impl = bool` - Should functions be generated for anonymous types? Equivalent to setting `anonymous_enum_impl` and `anonymous_struct_impl`
    * `named_enum_impl = bool` - Should variant access functions be generated for anonymous enums? Defaults to `false`
    * `named_struct_impl = bool` - Should member access functions be generated for anonymous structs? Defaults to `false`
    * `anonymous_enum_impl = bool` - Should member access functions be generated for anonymous enums? Defaults to `true`
    * `anonymous_struct_impl = bool` - Should member access functions be generated for anonymous structs? Defaults to `false`

# Dhall Input

Each .dhall file in the specified directory will be evaluated and must return a record, a union, a [schema](https://docs.dhall-lang.org/tutorials/Language-Tour.html#record-completion),
or a function taking a single `Type` parameter which returns one of the previous types.

# Generic Types

`dhall_types` supports generating types with a single type parameter when provided a Dhall file containing a function of type `Type -> Type`. 
The type parameter will be named `T`. Like non-generic types, member types will be interpreted as instances of generic types if possible. For example,  

### my_generic.dhall
``` dhall
    \(T: Type) -> {
      field: T
    }
```

### my_type.dhall
``` dhall
    let MyGeneric = ./my_generic.dhall
    {
      a: MyGeneric Natural,
      b: { field: Text },
    }
```

... becomes...

``` rust
    pub struct MyGeneric<T> {
      pub field: T,
    }
    pub struct MyType {
      a: MyGeneric<u64>,
      b: MyGeneric<String>,
    }
```

# Generated Functions

Anonymous enums have functions generated for each variant by default. If a variant `MyVariant` has an associated type `T`, the enum will have the following functions:
``` rust
fn my_variant(&self) -> Option<&T>
fn my_variant_mut(&mut self) -> Option<&mut T>
fn into_my_variant(self) -> Result<T, Self>
```

If the variant does not have an associated type, it will have the following function:
``` rust
fn is_my_variant(&self) -> bool
```

If struct functions are enabled, a field `my_field` of type `T` will have the following functions:
``` rust
fn my_field(&self) -> &T
fn my_field_mut(&mut self) -> &mut T
```

# Dhall Metadata

Because Dhall and Rust have fundamentally different type systems, it may be necessary to make some non-functional changes in the Dhall to generate Rust types as you intend.
Dhall's types are [structural](https://en.wikipedia.org/wiki/Structural_type_system), so two types with the same structure in two separate files are the same type as far as Dhall is concerned. This is considered an error when generating
Rust types, however, as it creates ambiguities when determining which Rust type should be used for a given Dhall type.  
To avoid the issue, you can assign 'name metadata' to your identical Dhall types so that they can be differentiated during evaluation. This will not affect your Dhall types in other
contexts - the resultant Dhall types are only modified when specific environment variables are set.  
The first method can be used with either records or unions and precedes the type:

### file_a.dhall
``` dhall
    env:rust_type ? (\(T: Type)->\(T: Type)->T) <MyEnumA>
    < A | B >
```

### file_b.dhall
``` dhall
    env:rust_type ? (\(T: Type)->\(T: Type)->T) <MyEnumB>
    < A | B >
```

... generates...

``` rust
    pub enum MyEnumA {
      A,
      B,
    }
    pub enum MyEnumB {
      A,
      B,
    }
```

The second method is less verbose, but can only be used for records and comes after the type:

``` dhall
    {
      name: Text
    } 
    //\\ (env:rust_type <MyStructA> ? {})

    {
      name: Text
    } 
    //\\ (env:rust_type <MyStructB> ? {})
```

... generates...

``` rust
    pub struct MyStructA { 
      pub name: String,
    }
    pub struct MyStructB {
      pub name: String,
    }
```

You can use `let` bindings to reduce the verbosity:

``` dhall
    let rust_type = env:rust_type ? (\(T: Type)->\(T: Type)->T)
    let rust_struct = \(T: Type) -> (env:rust_struct T ? {})
    let Abc = rust_type <Abc> < A | B | C >
    let Def = rust_type <Def> < D | E | F >
    let Ghi = { g: Bool, h: Bool, i: Bool } //\\ (rust_struct <Ghi>)
    let Jkl = { j: Bool, k: Bool, l: Bool } //\\ (rust_struct <Jkl>)
```

# Example
## Before

### ./dhall/schema/person.dhall

``` dhall
    {
        name: Text,
        age_range: <
            Baby        |
            Toddler     |
            Child       |
            Teenager    |
            Adult       |
            Senior
        >,
        occupation: Optional {
            title: Text,
            salary: Natural,
        }
    }
```

### ./src/dhall.rs
``` rust
    #[serde_dhall_typegen::dhall_types("./dhall/schema/")]
    mod dhall { }
```

## After

### ./src/dhall.rs (equivalent to)
```rust
    mod dhall {
        #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize)]
        pub struct Person {
            pub age_range: PersonAnon0,
            pub name: String,
            pub occupation: Option<PersonAnon1>,
        }
   
        #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize)]
        pub enum PersonAnon0 {
            Teenager,
            Senior,
            Baby,
            Toddler,
            Child,
            Adult,
        }
   
        impl PersonAnon0 {
            pub fn is_teenager(&self) -> bool {
                match self {
                    Self::Teenager => true,
                    _ => false,
                }
            }
            pub fn is_senior(&self) -> bool {
                match self {
                    Self::Senior => true,
                    _ => false,
                }
            }
            pub fn is_baby(&self) -> bool {
                match self {
                    Self::Baby => true,
                    _ => false,
                }
            }
            pub fn is_toddler(&self) -> bool {
                match self {
                    Self::Toddler => true,
                    _ => false,
                }
            }
            pub fn is_child(&self) -> bool {
                match self {
                    Self::Child => true,
                    _ => false,
                }
            }
            pub fn is_adult(&self) -> bool {
                match self {
                    Self::Adult => true,
                    _ => false,
                }
            }
        }
 
        #[derive(Debug, Clone, Eq, PartialEq, Hash, ::serde::Serialize, ::serde::Deserialize)]
        pub struct PersonAnon1 {
            pub salary: u64,
            pub title: String,
        }
    }
```

## Current Limitations

- All .dhall files in the provided directory must be valid and meet the type requirements, else compilation will fail.
- All generated structs have public members.
- Generated functions are all-or-nothing, you cannot exclude mutable access functions, for example.
- Attributes cannot be applied to the generated struct and enums (e.g. `#[non_exhaustive]`).
- The default values from a Dhall schema are ignored.
- Only a single type parameter per type is supported.
