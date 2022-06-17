let Thing1 = {
    name: Text,
} //\\ (env:rust_struct <Foo> ? {})
in
{
    Type = Thing1,
    default = { 
        name = "foo"
    }
}