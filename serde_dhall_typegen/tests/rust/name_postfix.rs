#[serde_dhall_typegen::dhall_types("../../../tests/dhall/schema/")]
mod dhall { }

fn main() {
    let foo = serde_dhall::from_str("{ name = \"foo\" }").parse::<dhall::Foo>().unwrap();
    let bar = serde_dhall::from_str("{ name = \"bar\" }").parse::<dhall::Bar>().unwrap();
    let foo_prime = dhall::Foo { name: "foo'".to_string() };
    let bar_prime = dhall::Bar { name: "bar'".to_string() };
    println!("{}", foo.name);
    println!("{}", bar.name);
    println!("{}", foo_prime.name);
    println!("{}", bar_prime.name);
}