let Person = {
    name: Text,
    age_range: <
        Baby        |
        Toddler     |
        Child       |
        Teenager    |
        Adult       |
        Senior
    >,
    occupation: Optional {
        title: Text,
        salary: Natural,
    }
}
in
{
    Type = Person,
    default = { 
        occupation = None {
            title: Text,
            salary: Natural,
        },
    }
}