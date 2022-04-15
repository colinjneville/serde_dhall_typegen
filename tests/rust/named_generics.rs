#[serde_dhall_typegen::dhall_types("../../../tests/dhall/schema/")]
mod dhall { }

fn main() {
    let pairs = dhall::Pairs {
        pair: dhall::Pair { first: 0, second: 1 },
        pair2: dhall::Pair2 { first: 2, second: 3 },
    };

    println!("({}, {}) & ({}, {})", pairs.pair.first, pairs.pair.second, pairs.pair2.first, pairs.pair2.second);
}

