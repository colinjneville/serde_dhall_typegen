#[serde_dhall_typegen::dhall_types("../../../tests/dhall/schema/")]
mod dhall { }

fn main() {
    let persons = serde_dhall::from_file("../../../tests/dhall/persons.dhall").parse::<Vec<dhall::Person>>().unwrap();
    for person in persons {
        println!("{} {:?} {:?}", person.name, person.age_range, person.occupation);
    }
}

