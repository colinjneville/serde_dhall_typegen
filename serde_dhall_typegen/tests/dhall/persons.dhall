let Person = ./schema/person.dhall
let AgeRange = < Baby | Toddler | Child | Teenager | Adult | Senior >
in
[
    Person::{
        name = "Abby",
        age_range = AgeRange.Adult,
        occupation = Some {
            title = "Accountant",
            salary = 80000,
        }
    },
    Person::{
        name = "Billy",
        age_range = AgeRange.Child,
    },
    Person::{
        name = "Carl",
        age_range = AgeRange.Senior,
        occupation = Some {
            title = "Clerk",
            salary = 40000,
        }
    },
]