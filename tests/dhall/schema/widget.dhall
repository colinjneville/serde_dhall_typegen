let rust_type = env:rust_type ? (\(t: Type)->\(t: Type)->t)
let Color =  rust_type <Color> < Red | Green | Blue > 
let Button = rust_type <Button> { color: Color, pressed: Bool }
in
{
    frame_color: Color,
    buttons: List Button
}