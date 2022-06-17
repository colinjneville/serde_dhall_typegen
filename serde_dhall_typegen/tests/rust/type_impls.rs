#[serde_dhall_typegen::dhall_types("../../../tests/dhall/schema/")]
mod dhall { 
    impl Person {
        pub fn can_view_pg13_rated_movies(&self) -> bool {
            self.age_range.is_teenager() ||
            self.age_range.is_adult() || 
            self.age_range.is_senior()
        }
    }

    impl ShoppingList {
        pub fn total_items(&self) -> u64 {
            let mut count = 0;
            for item in &self.items {
                count += item.quantity;
            }
            count
        }
    }
}

fn main() {
    let persons = serde_dhall::from_file("../../../tests/dhall/persons.dhall").parse::<Vec<dhall::Person>>().unwrap();
    for person in persons {
        if !person.can_view_pg13_rated_movies() {
            println!("{} is not allowed to see the movie", person.name);
        }
    }

    let list = serde_dhall::from_file("../../../tests/dhall/shopping_list.dhall").parse::<dhall::ShoppingList>().unwrap();
    println!("{} total items are on the list", list.total_items());
}