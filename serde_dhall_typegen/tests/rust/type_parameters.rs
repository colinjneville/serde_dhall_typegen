#[serde_dhall_typegen::dhall_types("../../../tests/dhall/schema/")]
mod dhall { }

fn main() {
    let list = serde_dhall::from_file("../../../tests/dhall/shopping_list.dhall").parse::<dhall::ShoppingList>().unwrap();
    for item in list.items {
        println!("{}x {}", item.quantity, item.item);
    }
}