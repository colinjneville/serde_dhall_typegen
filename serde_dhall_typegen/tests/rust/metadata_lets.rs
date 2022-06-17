#[serde_dhall_typegen::dhall_types("../../../tests/dhall/schema/")]
mod dhall { }

fn main() {
    let time = dhall::Time { hour: 11, minute: 59, second: 59, am_pm: dhall::AmPm::Am };
    println!("{}:{}:{} {:?}", time.hour, time.minute, time.second, time.am_pm);
}

