let rust_type = env:rust_type ? (\(T: Type)->\(T: Type)->T)
let rust_struct = \(T: Type) -> (env:rust_struct T ? {})
let AmPm = rust_type <AmPm> < Am | Pm >
in
{
    hour: Natural,
    minute: Natural,
    second: Natural,
    am_pm: AmPm,
} //\\ rust_struct <Time>
