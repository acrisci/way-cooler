use wlroots::{self, Area};

#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    Output(Output),
    Pointer(Pointer)
}

/// A representation of an Output for use in the Awesome module.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Output {
    pub name: String,
    pub effective_resolution: (i32, i32),
    pub focused: bool
}

impl Output {
    // TODO REMove
    pub fn new() -> Output {
        Output { name: "hi".into(),
                 effective_resolution: (1920, 1080),
                 focused: true }
    }
}

impl<'output> From<&'output mut wlroots::Output> for Output {
    fn from(output: &'output mut wlroots::Output) -> Self {
        let name = output.name();
        let effective_resolution = output.effective_resolution();
        // TODO
        let focused = true;
        Output { name,
                 effective_resolution,
                 focused }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Pointer {
    pub position: (f64, f64)
}

impl Pointer {
    /// Set the position of the pointer.
    pub fn set_position(&mut self, pos: (f64, f64)) {
        // TODO: post to the server
        self.position = pos
    }
}
