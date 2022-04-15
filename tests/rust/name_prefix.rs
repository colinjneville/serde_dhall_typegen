#[serde_dhall_typegen::dhall_types("../../../tests/dhall/schema/")]
mod dhall { 
    impl Button {
        pub fn new(color: Color) -> Self {
            Self {
                color,
                pressed: false,
            }
        }
    }
}

fn main() {
    let button0 = dhall::Button::new(dhall::Color::Red);
    let button1 = dhall::Button::new(dhall::Color::Blue);
    let button2 = dhall::Button::new(dhall::Color::Green);
    let widget = dhall::Widget {
        frame_color: dhall::Color::Red,
        buttons: vec![button0, button1, button2],
    };

    println!("The widget is {:?}!", widget.frame_color);
    for button in widget.buttons.iter() {
        println!("It has a {:?} button!", button.color);
    }
}

