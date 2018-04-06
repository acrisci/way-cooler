//! TODO Fill in

use super::{signal, XCB_CONNECTION_HANDLE};
use super::xproperty::{XProperty, XPropertyType, PROPERTIES};
use cairo::{self, ImageSurface};
use gdk_pixbuf::Pixbuf;
use glib::translate::ToGlibPtr;
use nix::{self, libc};
use rlua::{self, AnyUserData, LightUserData, Lua, MetaMethod, Table, ToLua, UserData,
           UserDataMethods, Value};
use std::{mem, ptr};
use std::default::Default;
use std::ffi::{CStr, CString};
use std::fmt::{self, Display, Formatter};
use std::process::{Command, Stdio};
use std::thread;
use xcb::{xkb, Connection};
use xcb::ffi::{self, xproto};

// TODO FIXME
// Often we are getting some raw pointers from the xcb replies
// we need to free them because the memory management for them is manual.

// TODO this isn't defined in the xcb crate, even though it should be.
// A patch should be open adding this to its generation scheme
extern "C" {
    fn xcb_xkb_get_names_value_list_unpack(buffer: *mut libc::c_void,
                                           nTypes: u8,
                                           indicators: u32,
                                           virtualMods: u16,
                                           groupNames: u8,
                                           nKeys: u8,
                                           nKeyAliases: u8,
                                           nRadioGroups: u8,
                                           which: u32,
                                           aux: *mut ffi::xkb::xcb_xkb_get_names_value_list_t)
                                           -> libc::c_int;
}

#[derive(Clone, Debug)]
pub struct AwesomeState {
    preferred_icon_size: u32
}

impl Default for AwesomeState {
    fn default() -> Self {
        AwesomeState { preferred_icon_size: 0 }
    }
}

impl Display for AwesomeState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Awesome: {:p}", self)
    }
}

impl UserData for AwesomeState {
    fn add_methods(methods: &mut UserDataMethods<Self>) {
        fn index<'lua>(_: &'lua Lua,
                       (awesome, index): (AnyUserData<'lua>, Value<'lua>))
                       -> rlua::Result<rlua::Value<'lua>> {
            let table = awesome.get_user_value::<Table>()?;
            table.get::<_, Value>(index)
        };
        methods.add_meta_function(MetaMethod::Index, index);
    }
}

pub fn init(lua: &Lua) -> rlua::Result<()> {
    let awesome_table = lua.create_table()?;
    method_setup(lua, &awesome_table)?;
    property_setup(lua, &awesome_table)?;
    let globals = lua.globals();
    let awesome = lua.create_userdata(AwesomeState::default())?;
    awesome.set_user_value(awesome_table)?;
    globals.set("awesome", awesome)
}

fn method_setup<'lua>(lua: &'lua Lua, awesome_table: &Table<'lua>) -> rlua::Result<()> {
    // TODO Fill in rest
    awesome_table.set("connect_signal",
                       lua.create_function(signal::global_connect_signal)?)?;
    awesome_table.set("disconnect_signal",
                       lua.create_function(signal::global_disconnect_signal)?)?;
    awesome_table.set("emit_signal",
                       lua.create_function(signal::global_emit_signal)?)?;
    awesome_table.set("xrdb_get_value", lua.create_function(xrdb_get_value)?)?;
    awesome_table.set("xkb_set_layout_group",
                       lua.create_function(xkb_set_layout_group)?)?;
    awesome_table.set("xkb_get_layout_group",
                       lua.create_function(xkb_get_layout_group)?)?;
    awesome_table.set("set_preferred_icon_size",
                       lua.create_function(set_preferred_icon_size)?)?;
    awesome_table.set("register_xproperty",
                       lua.create_function(register_xproperty)?)?;
    awesome_table.set("xkb_get_group_names",
                       lua.create_function(xkb_get_group_names)?)?;
    awesome_table.set("set_xproperty", lua.create_function(set_xproperty)?)?;
    awesome_table.set("get_xproperty", lua.create_function(get_xproperty)?)?;
    awesome_table.set("systray", lua.create_function(systray)?)?;
    awesome_table.set("restart", lua.create_function(restart)?)?;
    awesome_table.set("load_image", lua.create_function(load_image)?)?;
    awesome_table.set("sync", lua.create_function(sync)?)?;
    awesome_table.set("exec", lua.create_function(exec)?)?;
    awesome_table.set("kill", lua.create_function(kill)?)?;
    awesome_table.set("quit", lua.create_function(quit)?)
}

fn property_setup<'lua>(lua: &'lua Lua, awesome_table: &Table<'lua>) -> rlua::Result<()> {
    // TODO Do properly
    awesome_table.set("version", "0".to_lua(lua)?)?;
    awesome_table.set("themes_path", "/usr/share/awesome/themes".to_lua(lua)?)?;
    awesome_table.set("conffile", "".to_lua(lua)?)
}

/// Registers a new X property
fn register_xproperty<'lua>(_: &'lua Lua, (__rust, __type): (String, String)) -> rlua::Result<()> {
    Ok(())
}

/// Get layout short names
fn xkb_get_group_names<'lua>(lua: &'lua Lua, _: ()) -> rlua::Result<Value<'lua>> {
    "hello".to_lua(lua)
}

/// Query & set information about the systray
fn systray<'lua>(_: &'lua Lua, _: ()) -> rlua::Result<(u32, Value)> {
    Ok((0, Value::Nil))
}

/// Restart Awesome by restarting the Lua thread
fn restart<'lua>(_: &'lua Lua, _: ()) -> rlua::Result<()> {
    use lua::{self, LuaQuery};
    if let Err(err) = lua::send(LuaQuery::Restart) {
        warn!("Could not restart Lua thread {:#?}", err);
    }
    Ok(())
}

/// Load an image from the given path
/// Returns either a cairo surface as light user data, nil and an error message
fn load_image<'lua>(lua: &'lua Lua, file_path: String) -> rlua::Result<Value<'lua>> {
    let pixbuf = Pixbuf::new_from_file(file_path.as_str())
        .map_err(|err| rlua::Error::RuntimeError(format!("{}", err)))?;
    let surface = load_surface_from_pixbuf(pixbuf);
    // UGH, I wanted to do to_glib_full, but that isn't defined apparently
    // So now I have to ignore the lifetime completely and just forget about the
    // surface.
    let surface_ptr = surface.to_glib_none().0;
    ::std::mem::forget(surface);
    rlua::LightUserData(surface_ptr as _).to_lua(lua)
}

fn exec(_: &Lua, command: String) -> rlua::Result<()> {
    trace!("exec: \"{}\"", command);
    thread::Builder::new().name(command.clone())
                          .spawn(|| {
                                     Command::new(command).stdout(Stdio::null())
                                                          .spawn()
                                                          .expect("Could not spawn command")
                                 })
                          .expect("Unable to spawn thread");
    Ok(())
}

/// Kills a PID with the given signal
///
/// Returns false if it could not send the signal to that process
fn kill(_: &Lua, (pid, sig): (libc::pid_t, libc::c_int)) -> rlua::Result<bool> {
    Ok(nix::sys::signal::kill(pid, sig).is_ok())
}

fn set_preferred_icon_size(lua: &Lua, val: u32) -> rlua::Result<()> {
    let awesome_state = lua.globals().get::<_, AnyUserData>("awesome")?;
    let mut awesome_state = awesome_state.borrow_mut::<AwesomeState>()?;
    awesome_state.preferred_icon_size = val;
    Ok(())
}

fn quit(_: &Lua, _: ()) -> rlua::Result<()> {
    ::wlroots::terminate();
    Ok(())
}

/// No need to sync in Wayland
fn sync(_: &Lua, _: ()) -> rlua::Result<()> {
    Ok(())
}

fn set_xproperty(_: &Lua, _: Value) -> rlua::Result<()> {
    warn!("set_xproperty not supported");
    Ok(())
}

fn get_xproperty(_: &Lua, _: Value) -> rlua::Result<()> {
    warn!("get_xproperty not supported");
    Ok(())
}

fn xkb_set_layout_group(_: &Lua, _group: i32) -> rlua::Result<()> {
    warn!("xkb_set_layout_group not supported; Wait until wlroots");
    Ok(())
}

fn xkb_get_layout_group<'lua>(lua: &'lua Lua, _: ()) -> rlua::Result<Value<'lua>> {
    use xcb::ffi::xkb;
    unsafe {
        let raw_con = lua.named_registry_value::<LightUserData>(XCB_CONNECTION_HANDLE)?
                         .0 as _;
        let state_c = xkb::xcb_xkb_get_state_unchecked(raw_con, xkb::XCB_XKB_ID_USE_CORE_KBD as _);
        let state_r = xkb::xcb_xkb_get_state_reply(raw_con, state_c, ptr::null_mut());
        if state_r.is_null() {
            warn!("State reply was NULL");
            return Ok(Value::Nil)
        }
        (*state_r).group.to_lua(lua)
    }
}

fn xrdb_get_value(_lua: &Lua,
                  (_resource_class, _resource_name): (String, String))
                  -> rlua::Result<Value> {
    warn!("xrdb_get_value not supported");
    Ok(Value::Nil)
}

/// Using a Pixbuf buffer, loads the data into a Cairo surface.
pub fn load_surface_from_pixbuf(pixbuf: Pixbuf) -> ImageSurface {
    let width = pixbuf.get_width();
    let height = pixbuf.get_height();
    let channels = pixbuf.get_n_channels();
    let pix_stride = pixbuf.get_rowstride() as usize;
    // NOTE This is safe because we aren't modifying the bytes, but there's no
    // immutable view
    let pixels = unsafe { pixbuf.get_pixels() };
    let format = if channels == 3 {
        cairo::Format::Rgb24
    } else {
        cairo::Format::ARgb32
    };
    let mut surface =
        ImageSurface::create(format, width, height).expect("Could not create image of that size");
    let cairo_stride = surface.get_stride() as usize;
    {
        let mut cairo_data = surface.get_data().unwrap();
        let mut pix_pixels_index = 0;
        let mut cairo_pixels_index = 0;
        for _ in 0..height {
            let mut pix_pixels_index2 = pix_pixels_index;
            let mut cairo_pixels_index2 = cairo_pixels_index;
            for _ in 0..width {
                if channels == 3 {
                    let r = pixels[pix_pixels_index2];
                    let g = pixels[pix_pixels_index2 + 1];
                    let b = pixels[pix_pixels_index2 + 2];
                    cairo_data[cairo_pixels_index2] = b;
                    cairo_data[cairo_pixels_index2 + 1] = g;
                    cairo_data[cairo_pixels_index2 + 2] = r;
                    pix_pixels_index2 += 3;
                    // NOTE Four because of the alpha value we ignore
                    cairo_pixels_index2 += 4;
                } else {
                    let mut r = pixels[pix_pixels_index];
                    let mut g = pixels[pix_pixels_index + 1];
                    let mut b = pixels[pix_pixels_index + 2];
                    let a = pixels[pix_pixels_index + 3];
                    let alpha = a as f64 / 255.0;
                    r *= alpha as u8;
                    g *= alpha as u8;
                    b *= alpha as u8;
                    cairo_data[cairo_pixels_index] = b;
                    cairo_data[cairo_pixels_index + 1] = g;
                    cairo_data[cairo_pixels_index + 2] = r;
                    cairo_data[cairo_pixels_index + 3] = a;
                    pix_pixels_index2 += 4;
                    cairo_pixels_index2 += 4;
                }
            }
            pix_pixels_index += pix_stride;
            cairo_pixels_index += cairo_stride;
        }
    }
    surface
}
