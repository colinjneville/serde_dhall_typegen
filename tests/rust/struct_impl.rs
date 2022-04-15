#[serde_dhall_typegen::dhall_types("../../../tests/dhall/schema/",
    struct_impl = true,
)]
mod dhall { }

fn main() {
    let persons = serde_dhall::from_file("../../../tests/dhall/persons.dhall").parse::<Vec<dhall::Person>>().unwrap();
    for person in persons {
        let occupation_title = if let Some(occupation) = person.occupation() {
            occupation.title()
        } else {
            "Unemployed"
        };
        println!("{}: {}", person.name(), occupation_title);
    }
}