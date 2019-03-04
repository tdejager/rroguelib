#[macro_use]
extern crate glium;
extern crate rusttype;

use glium::{glutin};
mod fonts;

pub struct Roguelib {
    pub display: glium::Display,
    pub event_loop: glutin::EventsLoop
}

impl Roguelib {

    pub fn new<T: Into<String>>(title: T) -> Roguelib {

        let window = glutin::WindowBuilder::new()
            .with_dimensions((1920, 1080).into())
            .with_title(title);

        let context = glutin::ContextBuilder::new().with_vsync(true);
        let event_loop = glutin::EventsLoop::new();
        let display = glium::Display::new(window, context, &event_loop).unwrap();

        Roguelib{display, event_loop }
    }
}
