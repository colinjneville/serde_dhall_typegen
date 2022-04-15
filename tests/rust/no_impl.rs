#[serde_dhall_typegen::dhall_types("../../../tests/dhall/schema/",
    anonymous_enum_impl = false,
)]
mod dhall { }

fn main() {
    let persons = serde_dhall::from_file("../../../tests/dhall/persons.dhall").parse::<Vec<dhall::Person>>().unwrap();
    for person in persons {
        println!("{}", person.age_range.is_adult());
    }
}