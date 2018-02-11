// NOTE need to store the drawable in lua, because it's a reference to a drawable a lua object


use std::default::Default;
use std::fmt::{self, Display, Formatter};
use std::cell::RefCell;
use std::sync::{Mutex, MutexGuard, Arc, Weak};
use rustwlc::{Geometry, Point, Size};
use rlua::{self, Table, Lua, UserData, ToLua, Value};
use rlua::prelude::LuaInteger;
use super::drawable::Drawable;
use super::property::Property;
use cairo::ImageSurface;

use super::class::{self, Class, ClassBuilder};
use super::object::{Object, Objectable, ObjectBuilder};

lazy_static! {
    static ref DRAWINS: Mutex<RefCell<Vec<Weak<Mutex<DrawinSharedState>>>>> = Mutex::new(RefCell::new(Vec::default()));
}

#[derive(Clone, Debug)]
pub struct DrawinSharedState {
    ontop: bool,
    pub visible: bool,
    cursor: String,
    pub geometry: Geometry,
    geometry_dirty: bool,
    pub surface: Option<Arc<Mutex<ImageSurface>>>
}

#[derive(Clone, Debug)]
pub struct DrawinState {
    // Note that the drawable is stored in Lua.
    // TODO WINDOW_OBJECT_HEADER??
    pub state: Arc<Mutex<DrawinSharedState>>
}

#[derive(Clone, Debug)]
pub struct Drawin<'lua>(Table<'lua>);

impl DrawinState {
    fn lock(&self) -> rlua::Result<MutexGuard<DrawinSharedState>> {
        self.state.lock().map_err(// FIXME
                                  // |e| rlua::Error::external(e))
                                  |_| rlua::Error::CoroutineInactive)
    }

    // Collect the list of visible drawins into a vector
    pub fn collect_visible() -> Vec<Arc<Mutex<DrawinSharedState>>> {
        let list = DRAWINS.lock().unwrap();
        // Get a list of entries and at the same time remove dead entries
        let mut result = Vec::default();
        list.borrow_mut().retain(|ref e| {
            match e.upgrade() {
                Some(e) => {
                    let drawin = e.lock().unwrap();
                    if drawin.visible {
                        result.push(Arc::clone(&e));
                    }
                    true
                },
                None => false
            }
        });
        result
    }
}

impl UserData for DrawinState {}

impl Default for DrawinState {
    fn default() -> Self {
        let result = DrawinState {
            state: Arc::new(Mutex::new(DrawinSharedState {
                ontop: false,
                visible: false,
                cursor: String::default(),
                geometry: Geometry::zero(),
                geometry_dirty: false,
                surface: None
            }))
        };
        let list = DRAWINS.lock().unwrap();
        list.borrow_mut().push(Arc::downgrade(&result.state));
        result
    }
}

impl Display for DrawinState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Drawin: {:p}", self)
    }
}

impl <'lua> Drawin<'lua> {
    fn new(lua: &'lua Lua, args: Table) -> rlua::Result<Object<'lua>> {
        let class = class::class_setup(lua, "drawin")?;
        Ok(object_setup(lua, Drawin::allocate(lua, class)?)?
           .handle_constructor_argument(args)?
           .build())
    }

    fn update_drawing(&mut self) -> rlua::Result<()> {
        let state = self.state()?;
        let table = &self.0;
        let mut drawable = Drawable::cast(table.get::<_, Table>("drawable")?.into())?;
        let mut state = state.lock()?;
        drawable.set_geometry(state.geometry)?;
        state.surface = drawable.state()?.surface.clone(); // This clones Option<Arc<...>>, aka creates a new reference in the Arc
        table.raw_set::<_, Table>("drawable", drawable.get_table())?;
        Ok(())
    }

    fn get_visible(&mut self) -> rlua::Result<bool> {
        let state = self.state()?;
        let state = state.lock()?;
        Ok(state.visible)
    }

    fn set_visible(&mut self, val: bool) -> rlua::Result<()> {
        let drawin = self.state()?;
        drawin.lock()?.visible = val;
        self.map()?;
        self.set_state(drawin)
    }

    fn map(&mut self) -> rlua::Result<()> {
        // TODO other things
        self.update_drawing()
    }

    fn get_geometry(&self) -> rlua::Result<Geometry> {
        let state = self.state()?;
        let state = state.lock()?;
        Ok(state.geometry)
    }

    fn resize(&mut self, geometry: Geometry) -> rlua::Result<()> {
        let state = self.state()?;
        {
        let mut state = state.lock()?;
        let old_geometry = state.geometry;
        state.geometry = geometry;
        if state.geometry.size.w <= 0 {
            state.geometry.size.w = old_geometry.size.w;
        }
        if state.geometry.size.h <= 0 {
            state.geometry.size.h = old_geometry.size.h
        }
        state.geometry_dirty = true;
        // TODO emit signals
        // TODO update screen workareas like in awesome? Might not be necessary
        // TODO Currently have to call set_state() before update_drawing; change that
        }
        self.set_state(state)?;
        self.update_drawing()
    }
}

impl <'lua> ToLua<'lua> for Drawin<'lua> {
    fn to_lua(self, lua: &'lua Lua) -> rlua::Result<Value<'lua>> {
        self.0.to_lua(lua)
    }
}

impl_objectable!(Drawin, DrawinState);

pub fn init(lua: &Lua) -> rlua::Result<Class> {
    property_setup(lua, method_setup(lua, Class::builder(lua, "drawin", None, None, None)?)?)?
        .save_class("drawin")?
        .build()
}

fn method_setup<'lua>(lua: &'lua Lua, builder: ClassBuilder<'lua>) -> rlua::Result<ClassBuilder<'lua>> {
    // TODO Do properly
    use super::dummy;
    builder.method("connect_signal".into(), lua.create_function(dummy))?
            // TODO This should be adding properties, e.g like luaA_class_new
           .method("__call".into(), lua.create_function(|lua, (_, args): (Value, Table)| Drawin::new(lua, args)))
}

fn property_setup<'lua>(lua: &'lua Lua, builder: ClassBuilder<'lua>) -> rlua::Result<ClassBuilder<'lua>> {
    builder
        .property(Property::new("x".into(),
                                Some(lua.create_function(set_x)),
                                Some(lua.create_function(get_x)),
                                Some(lua.create_function(set_x))))?
        .property(Property::new("y".into(),
                                Some(lua.create_function(set_y)),
                                Some(lua.create_function(get_y)),
                                Some(lua.create_function(set_y))))?
        .property(Property::new("width".into(),
                                Some(lua.create_function(set_width)),
                                Some(lua.create_function(get_width)),
                                Some(lua.create_function(set_width))))?
        .property(Property::new("height".into(),
                                Some(lua.create_function(set_height)),
                                Some(lua.create_function(get_height)),
                                Some(lua.create_function(set_height))))?
        .property(Property::new("visible".into(),
                                Some(lua.create_function(set_visible)),
                                Some(lua.create_function(get_visible)),
                                Some(lua.create_function(set_visible))))
}

fn object_setup<'lua>(lua: &'lua Lua, builder: ObjectBuilder<'lua>) -> rlua::Result<ObjectBuilder<'lua>> {
    // TODO Do properly
    let table = lua.create_table();
    let drawable_table = Drawable::new(lua)?.to_lua(lua)?;
    table.set("drawable", drawable_table)?;
    table.set("geometry", lua.create_function(drawin_geometry))?;
    table.set("struts", lua.create_function(drawin_struts))?;
    builder.add_to_meta(table)
}

fn set_visible<'lua>(_: &'lua Lua, (table, visible): (Table<'lua>, bool))
                     -> rlua::Result<()> {
    let mut drawin = Drawin::cast(table.into())?;
    drawin.set_visible(visible)
    // TODO signal
}

fn get_visible<'lua>(_: &'lua Lua, table: Table<'lua>) -> rlua::Result<bool> {
    let mut drawin = Drawin::cast(table.into())?;
    drawin.get_visible()
    // TODO signal
}

fn drawin_geometry<'lua>(lua: &'lua Lua, (drawin, geometry): (Table<'lua>, Option<Table<'lua>>)) -> rlua::Result<Table<'lua>> {
    let mut drawin = Drawin::cast(drawin.into())?;
    if let Some(geometry) = geometry {
        let w = geometry.get::<_, i32>("width")?;
        let h = geometry.get::<_, i32>("height")?;
        let x = geometry.get::<_, i32>("x")?;
        let y = geometry.get::<_, i32>("y")?;
        if w > 0 && h > 0 {
            let geo = Geometry {
                origin: Point { x, y },
                size: Size { w: w as u32, h: h as u32 }
            };
            drawin.resize(geo)?;
        }
    }
    let new_geo = drawin.get_geometry()?;
    let Size { w, h } = new_geo.size;
    let Point { x, y } = new_geo.origin;
    let res = lua.create_table();
    res.set("x", x)?;
    res.set("y", y)?;
    res.set("height", h)?;
    res.set("width", w)?;
    Ok(res)
}

fn get_x<'lua>(_: &'lua Lua, drawin: Table<'lua>) -> rlua::Result<LuaInteger> {
    let drawin = Drawin::cast(drawin.into())?;
    let Point { x, .. } = drawin.get_geometry()?.origin;
    Ok(x as LuaInteger)
}

fn set_x<'lua>(_: &'lua Lua, (drawin, x): (Table<'lua>, LuaInteger)) -> rlua::Result<()> {
    let mut drawin = Drawin::cast(drawin.into())?;
    let mut geo = drawin.get_geometry()?;
    geo.origin.x = x as i32;
    drawin.resize(geo)?;
    Ok(())
}

fn get_y<'lua>(_: &'lua Lua, drawin: Table<'lua>) -> rlua::Result<LuaInteger> {
    let drawin = Drawin::cast(drawin.into())?;
    let Point { y, .. } = drawin.get_geometry()?.origin;
    Ok(y as LuaInteger)
}

fn set_y<'lua>(_: &'lua Lua, (drawin, y): (Table<'lua>, LuaInteger)) -> rlua::Result<()> {
    let mut drawin = Drawin::cast(drawin.into())?;
    let mut geo = drawin.get_geometry()?;
    geo.origin.y = y as i32;
    drawin.resize(geo)?;
    Ok(())
}

fn get_width<'lua>(_: &'lua Lua, drawin: Table<'lua>) -> rlua::Result<LuaInteger> {
    let drawin = Drawin::cast(drawin.into())?;
    let Size { w, .. } = drawin.get_geometry()?.size;
    Ok(w as LuaInteger)
}

fn set_width<'lua>(_: &'lua Lua, (drawin, width): (Table<'lua>, LuaInteger)) -> rlua::Result<()> {
    let mut drawin = Drawin::cast(drawin.into())?;
    let mut geo = drawin.get_geometry()?;
    if width > 0 {
        geo.size.w = width as u32;
        drawin.resize(geo)?;
    }
    Ok(())
}

fn get_height<'lua>(_: &'lua Lua, drawin: Table<'lua>) -> rlua::Result<LuaInteger> {
    let drawin = Drawin::cast(drawin.into())?;
    let Size { h, .. } = drawin.get_geometry()?.size;
    Ok(h as LuaInteger)
}

fn set_height<'lua>(_: &'lua Lua, (drawin, height): (Table<'lua>, LuaInteger)) -> rlua::Result<()> {
    let mut drawin = Drawin::cast(drawin.into())?;
    let mut geo = drawin.get_geometry()?;
    if height > 0 {
        geo.size.h = height as u32;
        drawin.resize(geo)?;
    }
    Ok(())
}

fn drawin_struts<'lua>(lua: &'lua Lua, _drawin: Table<'lua>) -> rlua::Result<Table<'lua>> {
    // TODO: Implement this properly. Struts means this drawin reserves some space on the screen
    // that it is visible on, shrinking the workarea in the specified directions.
    let res = lua.create_table();
    res.set("left", 0)?;
    res.set("right", 0)?;
    res.set("top", 0)?;
    res.set("bottom", 0)?;
    Ok(res)
}
