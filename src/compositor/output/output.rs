use cairo::*;
use compositor::{self, Server, Shell};
use std::time::{SystemTime, UNIX_EPOCH};
use wlroots::{self, project_box, Area, Compositor, Origin, OutputHandler, Size,
              WL_SHM_FORMAT_ARGB8888};

pub struct Output;

impl OutputHandler for Output {
    fn on_frame(&mut self, compositor: &mut Compositor, output: &mut wlroots::Output) {
        let state: &mut Server = compositor.data.downcast_mut().unwrap();
        let Server { ref mut layout,
                     ref mut views,
                     .. } = *state;

        let drawins = ::awesome::drawin::DrawinState::collect_visible();
        let renderer = compositor.renderer.as_mut().expect("gles2 disabled");
        for drawin in drawins {
            let drawin = drawin.lock().unwrap();
            if drawin.visible {
                if let Some(surface) = drawin.surface.as_ref() {
                    let mut surface = surface.lock().unwrap();
                    let mut other = ImageSurface::create(Format::ARgb32,
                                                         surface.get_width(),
                                                         surface.get_height()).unwrap();
                    {
                        let cr = Context::new(&other);
                        cr.set_source_surface(&*surface, 0.0, 0.0);
                        cr.paint();
                    }
                    let Area { size: Size { width, height },
                               .. } = drawin.geometry;
                    let texture = renderer.create_texture_from_pixels(WL_SHM_FORMAT_ARGB8888,
                                                                      (width * 4) as _,
                                                                      width as _,
                                                                      height as _,
                                                                      &mut *other.get_data()
                                                                                 .unwrap())
                                          .unwrap();
                    let mut renderer = renderer.render(output, None);
                    renderer.clear([0.25, 0.25, 0.25, 1.0]);
                    let transform = renderer.output.get_transform().invert();
                    let matrix = project_box(drawin.geometry,
                                             transform,
                                             0.0,
                                             renderer.output.transform_matrix());
                    renderer.render_texture_with_matrix(&texture, matrix);
                    for view in { &mut *views } {
                        let mut surface = view.shell.surface();
                        run_handles!([(surface: {surface}),
                          (layout: {&mut *layout})] => {
                let (width, height) = surface.current_state().size();
                let (render_width, render_height) =
                    (width * renderer.output.scale() as i32,
                     height * renderer.output.scale() as i32);
                let render_box = Area::new(view.origin,
                                           Size::new(render_width,
                                                     render_height));
                if layout.intersects(renderer.output, render_box) {
                    let transform = renderer.output.get_transform().invert();
                    let matrix = project_box(render_box,
                                             transform,
                                             0.0,
                                             renderer.output
                                             .transform_matrix());
                    renderer.render_texture_with_matrix(&surface.texture(),
                                                        matrix);
                    let start = SystemTime::now();
                    let now = start.duration_since(UNIX_EPOCH)
                        .expect("Time went backwards");
                    surface.send_frame_done(now);
                }
            })
                .expect("Surface was destroyed")
                .expect("Layout was destroyed")
                    }
                }
            }
        }
    }
}
