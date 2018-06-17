use compositor::Shell;
use std::cell::Cell;
use wlroots::XdgV6ShellState::*;
use wlroots::{Origin, SurfaceHandle};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct View {
    pub shell: Shell,
    pub origin: Cell<Origin>,
    pub lua_id: u32
}

static mut lua_counter: u32 = 0;

impl View {
    pub fn new(shell: Shell) -> View {
        let lua_id = unsafe {
            lua_counter += 1;
            lua_counter
        };
        View { shell: shell,
               origin: Cell::new(Origin::default()),
               lua_id
        }
    }

    pub fn surface(&self) -> SurfaceHandle {
        match self.shell {
            Shell::XdgV6(ref xdg_surface) => {
                with_handles!([(xdg_surface: {xdg_surface})] => {
                    xdg_surface.surface()
                }).unwrap()
            }
        }
    }

    pub fn activate(&self, activate: bool) {
        match self.shell {
            Shell::XdgV6(ref xdg_surface) => {
                with_handles!([(xdg_surface: {xdg_surface})] => {
                    match xdg_surface.state() {
                        Some(&mut TopLevel(ref mut toplevel)) => {
                            toplevel.set_activated(activate);
                        },
                        _ => unimplemented!()
                    }
                }).unwrap();
            }
        }
    }

    pub fn for_each_surface(&self, f: &mut FnMut(SurfaceHandle, i32, i32)) {
        match self.shell {
            Shell::XdgV6(ref xdg_surface) => {
                with_handles!([(xdg_surface: {xdg_surface})] => {
                    xdg_surface.for_each_surface(f);
                }).unwrap();
            }
        }
    }

    pub fn title(&self) -> String {
        match self.shell {
            Shell::XdgV6(ref xdg_surface) => {
                with_handles!([(xdg_surface: {xdg_surface})] => {
                    match xdg_surface.state() {
                        Some(&mut TopLevel(ref mut toplevel)) => {
                            toplevel.title()
                        },
                        _ => unimplemented!()
                    }
                }).unwrap()
            }
        }
    }
}
