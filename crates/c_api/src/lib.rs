#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::f64::consts::PI;
use std::ffi::{c_char, CStr};
use sparse_primitives::color::{AlphaColor, Srgb};
use sparse_primitives::{FillRule, Pixmap, RenderContext};
use sparse_primitives::kurbo::{Affine, BezPath, Point, Rect, RoundedRect, RoundedRectRadii, Shape};
use sparse_primitives::paint::Paint;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct sp_point {
    x: f64,
    y: f64,
}

impl From<Point> for sp_point {
    fn from(value: Point) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct sp_transform {
    sx: f64,
    kx: f64,
    ky: f64,
    sy: f64,
    tx: f64,
    ty: f64,
}

impl From<Affine> for sp_transform {
    fn from(value: Affine) -> Self {
        let components = value.as_coeffs();
        Self {
            sx: components[0],
            kx: components[1],
            ky: components[2],
            sy: components[3],
            tx: components[4],
            ty: components[5],
        }
    }
}

impl From<sp_transform> for Affine {
    fn from(value: sp_transform) -> Self {
        Affine::new([value.sx,
            value.kx,
            value.ky,
            value.sy,
            value.tx,
            value.ty,])
    }
}

pub struct sp_path(BezPath);

#[repr(C)]
#[derive(Debug)]
pub struct sp_color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<sp_color> for AlphaColor<Srgb> {
    fn from(value: sp_color) -> Self {
        Self::from_rgba8(value.r, value.g, value.b, value.a)
    }
}

#[repr(C)]
pub enum sp_fill_rule {
    Winding,
    EvenOdd,
}

impl From<sp_fill_rule> for FillRule {
    fn from(value: sp_fill_rule) -> Self {
        match value {
            sp_fill_rule::Winding => FillRule::NonZero,
            sp_fill_rule::EvenOdd => FillRule::EvenOdd,
        }
    }
}

#[repr(C)]
pub struct sp_rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl From<sp_rect> for Rect {
    fn from(value: sp_rect) -> Self {
        Rect::from_points((value.x0, value.y0), (value.x1, value.y1))
    }
}

#[no_mangle]
pub extern "C" fn sp_transform_identity() -> sp_transform {
    sp_transform {
        sx: 1.0,
        kx: 0.0,
        ky: 0.0,
        sy: 1.0,
        tx: 0.0,
        ty: 0.0,
    }
}

#[no_mangle]
pub extern "C" fn sp_transform_scale(sx: f64, sy: f64) -> sp_transform {
    sp_transform {
        sx,
        kx: 0.0,
        ky: 0.0,
        sy,
        tx: 0.0,
        ty: 0.0,
    }
}

#[no_mangle]
pub extern "C" fn sp_transform_translate(tx: f64, ty: f64) -> sp_transform {
    sp_transform {
        sx: 1.0,
        kx: 0.0,
        ky: 0.0,
        sy: 1.0,
        tx,
        ty,
    }
}

#[no_mangle]
pub extern "C" fn sp_transform_rotate(angle: f64) -> sp_transform {
    Affine::rotate(angle * PI / 180.0).into()
}

#[no_mangle]
pub extern "C" fn sp_transform_rotate_at(angle: f64, cx: f64, cy: f64) -> sp_transform {
    Affine::rotate_about(angle * PI / 180.0, Point::new(cx, cy)).into()
}

#[no_mangle]
pub unsafe extern "C" fn sp_path_create() -> *mut sp_path {
    Box::into_raw(Box::new(sp_path(BezPath::new())))
}

#[no_mangle]
pub unsafe extern "C" fn sp_move_to(p: *mut sp_path, x: f64, y: f64) {
    (*p).0.move_to((x, y));
}

#[no_mangle]
pub unsafe extern "C" fn sp_line_to(p: *mut sp_path, x: f64, y: f64) {
    (*p).0.line_to((x, y));
}

#[no_mangle]
pub unsafe extern "C" fn sp_quad_to(p: *mut sp_path, x0: f64, y0: f64, x1: f64, y1: f64) {
    (*p).0.quad_to((x0, y0), (x1, y1));
}

#[no_mangle]
pub unsafe extern "C" fn sp_cubic_to(
    p: *mut sp_path,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) {
    (*p).0.curve_to((x0, y0), (x1, y1), (x2, y2));
}

#[no_mangle]
pub unsafe extern "C" fn sp_close(p: *mut sp_path) {
    (*p).0.close_path();
}

#[no_mangle]
pub unsafe extern "C" fn sp_rounded_rect(rect: sp_rect, r: f64) -> *mut sp_path {
    let mut rect: Rect = rect.into();
    let mut rounded = rect.to_rounded_rect(RoundedRectRadii::from(r));

    Box::into_raw(Box::new(sp_path(rounded.to_path(0.1))))
}

#[no_mangle]
pub unsafe extern "C" fn ts_path_destroy(b: *mut sp_path) {
    let _ = Box::from_raw(b);
}

pub struct sp_context(RenderContext);

#[no_mangle]
pub unsafe extern "C" fn sp_context_create(width: u32, height: u32) -> *mut sp_context {
    let ctx = RenderContext::new(width as usize, height as usize);
    Box::into_raw(Box::new(sp_context(ctx)))
}

#[no_mangle]
pub unsafe extern "C" fn sp_context_destroy(ctx: *mut sp_context) {
    let _ = Box::from_raw(ctx);
}

pub struct sp_pixmap(Pixmap);

#[no_mangle]
pub unsafe extern "C" fn sp_pixmap_create(width: u32, height: u32) -> *mut sp_pixmap {
    let pixmap = Pixmap::new(width as usize, height as usize);
    Box::into_raw(Box::new(sp_pixmap(pixmap)))
}

#[no_mangle]
pub unsafe extern "C" fn sp_pixmap_destroy(pixmap: *mut sp_pixmap) {
    let _ = Box::from_raw(pixmap);
}

#[no_mangle]
pub unsafe extern "C" fn sp_render_to_pixmap(pixmap: *mut sp_pixmap, context: *mut sp_context) {
    (*context).0.render_to_pixmap(&mut (*pixmap).0);
}

#[no_mangle]
pub unsafe extern "C" fn sp_set_transform(
    ctx: *mut sp_context,
    transform: sp_transform,
) {
    (*ctx)
        .0
        .set_transform(transform.into())
}

#[no_mangle]
pub unsafe extern "C" fn sp_fill_path(
    ctx: *mut sp_context,
    path: *const sp_path,
    paint: sp_paint,
    fill_rule: sp_fill_rule,
) {
    let paint = convert_paint(paint);

    (*ctx)
        .0
        .fill_path(&(*path).0, fill_rule.into(), paint);
}

#[no_mangle]
pub unsafe extern "C" fn sp_fill_rect(
    pixmap: *mut sp_context,
    rect: sp_rect,
    paint: sp_paint,
) {
    let paint = convert_paint(paint);

    (*pixmap)
        .0
        .fill_rect(&rect.into(), paint);
}

//
// pub struct sp_argb(Vec<u8>);
//
// #[no_mangle]
// pub unsafe extern "C" fn sp_data(pixmap: *const sp_context) -> *mut sp_argb {
//     let mut buffer = Vec::with_capacity((*pixmap).0.data().len());
//
//     for pixel in (*pixmap).0.pixels() {
//         // let pixel = pixel.demultiply();
//         // buffer.push(data[3]);
//         buffer.extend_from_slice(&[pixel.blue(), pixel.green(), pixel.red(), pixel.alpha()]);
//     }
//
//     Box::into_raw(Box::new(sp_argb(buffer)))
// }
//
// #[no_mangle]
// pub unsafe extern "C" fn ts_argb_data(data: *const sp_argb) -> *const u8 {
//     (*data).0.as_ptr()
// }
//
// #[no_mangle]
// pub unsafe extern "C" fn ts_argb_destroy(data: *mut sp_argb) {
//     let _ = Box::from_raw(data);
// }
//
// #[repr(C)]
// pub struct ts_stroke {
//     width: f32,
// }
//
// impl From<ts_stroke> for Stroke {
//     fn from(value: ts_stroke) -> Self {
//         Self {
//             width: value.width,
//             ..Default::default()
//         }
//     }
// }
//
// #[no_mangle]
// pub unsafe extern "C" fn ts_pixmap_stroke_path(
//     pixmap: *mut sp_context,
//     path: *const sp_path,
//     transform: sp_transform,
//     paint: sp_paint,
//     stroke: ts_stroke,
//     blend_mode: ts_blend_mode,
// ) {
//     let paint = convert_paint(paint, blend_mode);
//
//     (*pixmap)
//         .0
//         .stroke_path(&(*path).0, &paint, &stroke.into(), transform.into(), None);
// }
//
#[repr(C)]
pub enum sp_paint {
    Color(sp_color),
}

unsafe fn convert_paint(paint: sp_paint) -> Paint {
    match paint {
        sp_paint::Color(color) => {
            let c: AlphaColor<Srgb> = color.into();
            c.into()
        }
    }
}
//
// #[no_mangle]
// pub unsafe extern "C" fn ts_pixmap_stroke_rect(
//     pixmap: *mut sp_context,
//     rect: sp_rect,
//     transform: sp_transform,
//     paint: sp_paint,
//     stroke: ts_stroke,
//     blend_mode: ts_blend_mode,
// ) {
//     let paint = convert_paint(paint, blend_mode);
//
//     (*pixmap).0.stroke_path(
//         &PathBuilder::from_rect(rect.into()),
//         &paint,
//         &stroke.into(),
//         transform.into(),
//         None,
//     );
// }
//
// trait PathBuilderExt {
//     fn arc_to(
//         &mut self,
//         rx: f32,
//         ry: f32,
//         x_axis_rotation: f32,
//         large_arc: bool,
//         sweep: bool,
//         x: f32,
//         y: f32,
//     );
// }
//
// impl PathBuilderExt for PathBuilder {
//     fn arc_to(
//         &mut self,
//         rx: f32,
//         ry: f32,
//         x_axis_rotation: f32,
//         large_arc: bool,
//         sweep: bool,
//         x: f32,
//         y: f32,
//     ) {
//         let prev = match self.last_point() {
//             Some(v) => v,
//             None => return,
//         };
//
//         let svg_arc = kurbo::SvgArc {
//             from: kurbo::Point::new(prev.x as f64, prev.y as f64),
//             to: kurbo::Point::new(x as f64, y as f64),
//             radii: kurbo::Vec2::new(rx as f64, ry as f64),
//             x_rotation: (x_axis_rotation as f64).to_radians(),
//             large_arc,
//             sweep,
//         };
//
//         match kurbo::Arc::from_svg_arc(&svg_arc) {
//             Some(arc) => {
//                 arc.to_cubic_beziers(0.1, |p1, p2, p| {
//                     self.cubic_to(
//                         p1.x as f32,
//                         p1.y as f32,
//                         p2.x as f32,
//                         p2.y as f32,
//                         p.x as f32,
//                         p.y as f32,
//                     );
//                 });
//             }
//             None => {
//                 self.line_to(x, y);
//             }
//         }
//     }
// }
//
// #[repr(C)]
// pub struct ts_gradient_stop {
//     pos: f32,
//     color: sp_color,
// }
//
// impl Into<GradientStop> for ts_gradient_stop {
//     fn into(self) -> GradientStop {
//         GradientStop::new(self.pos, self.color.into())
//     }
// }
//
// #[derive(Clone)]
// pub struct ts_linear_gradient {
//     x0: f32,
//     y0: f32,
//     x1: f32,
//     y1: f32,
//     stops: Vec<GradientStop>,
//     spread_mode: SpreadMode,
//     transform: sp_transform,
// }
//
// impl From<ts_linear_gradient> for Shader<'static> {
//     fn from(value: ts_linear_gradient) -> Self {
//         LinearGradient::new(
//             Point::from_xy(value.x0, value.y0),
//             Point::from_xy(value.x1, value.y1),
//             value.stops,
//             value.spread_mode,
//             value.transform.into(),
//         )
//             .unwrap()
//     }
// }
//
// #[derive(Clone)]
// pub struct ts_radial_gradient {
//     x0: f32,
//     y0: f32,
//     x1: f32,
//     y1: f32,
//     r0: f32,
//     stops: Vec<GradientStop>,
//     spread_mode: SpreadMode,
//     transform: sp_transform,
// }
//
// impl From<ts_radial_gradient> for Shader<'static> {
//     fn from(value: ts_radial_gradient) -> Self {
//         RadialGradient::new(
//             Point::from_xy(value.x0, value.y0),
//             Point::from_xy(value.x1, value.y1),
//             value.r0,
//             value.stops,
//             value.spread_mode,
//             value.transform.into(),
//         )
//             .unwrap()
//     }
// }
//
// #[no_mangle]
// pub unsafe extern "C" fn ts_linear_gradient_create(
//     x0: f32,
//     y0: f32,
//     x1: f32,
//     y1: f32,
//     spread_mode: ts_spread_mode,
//     transform: sp_transform,
// ) -> *mut ts_linear_gradient {
//     Box::into_raw(Box::new(ts_linear_gradient {
//         x0,
//         y0,
//         x1,
//         y1,
//         stops: vec![],
//         spread_mode: spread_mode.into(),
//         transform,
//     }))
// }
//
// #[no_mangle]
// pub unsafe extern "C" fn ts_radial_gradient_create(
//     x0: f32,
//     y0: f32,
//     x1: f32,
//     y1: f32,
//     r0: f32,
//     spread_mode: ts_spread_mode,
//     transform: sp_transform,
// ) -> *mut ts_radial_gradient {
//     Box::into_raw(Box::new(ts_radial_gradient {
//         x0,
//         y0,
//         x1,
//         y1,
//         r0,
//         stops: vec![],
//         spread_mode: spread_mode.into(),
//         transform,
//     }))
// }
//
// #[no_mangle]
// pub unsafe extern "C" fn ts_linear_gradient_push_stop(
//     g: *mut ts_linear_gradient,
//     stop: ts_gradient_stop,
// ) {
//     (*g).stops.push(stop.into())
// }
//
// #[no_mangle]
// pub unsafe extern "C" fn ts_radial_gradient_push_stop(
//     g: *mut ts_radial_gradient,
//     stop: ts_gradient_stop,
// ) {
//     (*g).stops.push(stop.into())
// }
//
// #[no_mangle]
// pub unsafe extern "C" fn ts_paint_destroy(paint: sp_paint) {
//     match paint {
//         sp_paint::Color(_) => {}
//         sp_paint::LinearGradient(l) => {
//             let _ = Box::from_raw(l);
//         }
//         sp_paint::RadialGradient(r) => {
//             let _ = Box::from_raw(r);
//         }
//     }
// }
//
// #[repr(C)]
// #[derive(Clone, Copy)]
// pub enum ts_spread_mode {
//     Repeat,
//     Pad,
//     Reflect
// }
//
// impl From<ts_spread_mode> for SpreadMode {
//     fn from(value: ts_spread_mode) -> Self {
//         match value {
//             ts_spread_mode::Repeat => SpreadMode::Repeat,
//             ts_spread_mode::Pad => SpreadMode::Pad,
//             ts_spread_mode::Reflect => SpreadMode::Reflect
//         }
//     }
// }