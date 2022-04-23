#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.pass("tests/rust/anonymous_types.rs");
    t.pass("tests/rust/metadata_lets.rs");
    t.pass("tests/rust/name_prefix.rs");
    t.pass("tests/rust/name_postfix.rs");
    t.pass("tests/rust/named_generics.rs");
    t.pass("tests/rust/struct_impl.rs");
    t.pass("tests/rust/type_impls.rs");
    t.pass("tests/rust/type_parameters.rs");
    
    // Negative tests disabled for now, anonymous type names are non-deterministic, and trybuild only allows exact matches for errors
    // t.compile_fail("tests/rust/no_impl.rs");
}