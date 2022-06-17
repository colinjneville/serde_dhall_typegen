let ShoppingList = ./schema/shopping_list.dhall
in
ShoppingList::{
    items = [
        {
            item = "Milk",
            quantity = 1,
        },
        {
            item = "Eggs",
            quantity = 12,
        },
        {
            item = "Bread",
            quantity = 2,
        },
        {
            item = "Bananas",
            quantity = 100,
        },
    ]
}