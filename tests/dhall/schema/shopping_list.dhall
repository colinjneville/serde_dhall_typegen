let Quantity = ./quantity.dhall
let ShoppingList = {
    items: List (Quantity Text)
}
in
{
    Type = ShoppingList,
    default = { items = []: List (Quantity Text)}
}
