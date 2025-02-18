#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use sparse_primitives::color::{AlphaColor, Srgb};
use sparse_primitives::kurbo::{
    Affine, BezPath, Cap, Join, Point, Rect, RoundedRectRadii, Shape, Stroke,
};
use sparse_primitives::paint::Paint;
use sparse_primitives::{FillRule, Pixmap, RenderContext};
use std::f64::consts::PI;

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

impl From<sp_point> for Point {
    fn from(value: sp_point) -> Self {
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
        Affine::new([value.sx, value.kx, value.ky, value.sy, value.tx, value.ty])
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
    Affine::rotate(angle).into()
}

#[no_mangle]
pub extern "C" fn sp_transform_rotate_at(angle: f64, cx: f64, cy: f64) -> sp_transform {
    Affine::rotate_about(angle, Point::new(cx, cy)).into()
}

#[no_mangle]
pub unsafe extern "C" fn sp_path_create() -> *mut sp_path {
    Box::into_raw(Box::new(sp_path(BezPath::new())))
}

#[no_mangle]
pub unsafe extern "C" fn sp_move_to(path: *mut sp_path, p: sp_point) {
    (*path).0.move_to(p);
}

#[no_mangle]
pub unsafe extern "C" fn sp_line_to(path: *mut sp_path, p: sp_point) {
    (*path).0.line_to(p);
}

#[no_mangle]
pub unsafe extern "C" fn sp_quad_to(path: *mut sp_path, p0: sp_point, p1: sp_point) {
    (*path).0.quad_to(p0, p1);
}

#[no_mangle]
pub unsafe extern "C" fn sp_cubic_to(path: *mut sp_path, p0: sp_point, p1: sp_point, p2: sp_point) {
    (*path).0.curve_to(p0, p1, p2);
}

#[no_mangle]
pub unsafe extern "C" fn sp_close(path: *mut sp_path) {
    (*path).0.close_path();
}

#[no_mangle]
pub unsafe extern "C" fn sp_rounded_rect(rect: sp_rect, r: f64) -> *mut sp_path {
    let rect: Rect = rect.into();
    let rounded = rect.to_rounded_rect(RoundedRectRadii::from(r));

    Box::into_raw(Box::new(sp_path(rounded.to_path(0.1))))
}

#[no_mangle]
pub unsafe extern "C" fn sp_path_destroy(b: *mut sp_path) {
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
pub unsafe extern "C" fn sp_set_transform(ctx: *mut sp_context, transform: sp_transform) {
    (*ctx).0.set_transform(transform.into())
}

#[no_mangle]
pub unsafe extern "C" fn sp_fill_path(
    ctx: *mut sp_context,
    path: *const sp_path,
    paint: sp_paint,
    fill_rule: sp_fill_rule,
) {
    let paint = convert_paint(paint);

    (*ctx).0.fill_path(&(*path).0, fill_rule.into(), paint);
}

#[no_mangle]
pub unsafe extern "C" fn sp_stroke_path(
    ctx: *mut sp_context,
    path: *const sp_path,
    paint: sp_paint,
    stroke: sp_stroke,
) {
    let paint = convert_paint(paint);

    (*ctx).0.stroke_path(&(*path).0, &stroke.into(), paint);
}

#[no_mangle]
pub unsafe extern "C" fn sp_fill_rect(ctx: *mut sp_context, rect: sp_rect, paint: sp_paint) {
    let paint = convert_paint(paint);

    (*ctx).0.fill_rect(&rect.into(), paint);
}

pub struct sp_argb(Vec<u8>);

#[no_mangle]
pub unsafe extern "C" fn sp_data(pixmap: *mut sp_pixmap) -> *mut sp_argb {
    let mut buffer = Vec::with_capacity((*pixmap).0.data().len());

    // (*pixmap).0.unpremultiply();

    for pixel in (*pixmap).0.data().chunks_exact(4) {
        buffer.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }

    Box::into_raw(Box::new(sp_argb(buffer)))
}

#[no_mangle]
pub unsafe extern "C" fn sp_argb_data(data: *const sp_argb) -> *const u8 {
    (*data).0.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn sp_argb_destroy(data: *mut sp_argb) {
    let _ = Box::from_raw(data);
}

#[repr(C)]
pub struct sp_stroke {
    width: f64,
}

impl From<sp_stroke> for Stroke {
    fn from(value: sp_stroke) -> Self {
        Self {
            width: value.width as f64,
            join: Join::Bevel,
            start_cap: Cap::Butt,
            end_cap: Cap::Butt,
            ..Default::default()
        }
    }
}

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

#[no_mangle]
pub unsafe extern "C" fn sp_stroke_rect(
    ctx: *mut sp_context,
    rect: sp_rect,
    paint: sp_paint,
    stroke: sp_stroke,
) {
    let paint = convert_paint(paint);

    (*ctx).0.stroke_rect(&rect.into(), &stroke.into(), paint);
}

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
