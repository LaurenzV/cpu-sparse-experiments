use image::{load_from_memory, Rgba, RgbaImage};
use once_cell::sync::Lazy;
use peniko::color::palette;
use peniko::kurbo::{Rect, Shape};
use sparse_primitives::execute::ExecutionMode;
use sparse_primitives::{Pixmap, RenderContext};
use std::cmp::max;
use std::path::PathBuf;
use std::sync::LazyLock;

const REPLACE: bool = true;

static REFS_PATH: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("refs"));
static DIFFS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("diffs");
    let _ = std::fs::remove_dir_all(&path);
    let _ = std::fs::create_dir_all(&path);
    path
});

pub fn get_ctx(width: usize, height: usize, transparent: bool) -> RenderContext {
    let mut execution_mode = ExecutionMode::Scalar;

    #[cfg(all(target_arch = "aarch64", feature = "simd"))]
    if option_env!("NEON").is_some() {
        execution_mode = ExecutionMode::Neon;
    }

    #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    if option_env!("AVX2").is_some() {
        execution_mode = ExecutionMode::Avx2;
    }

    let mut ctx = RenderContext::new_with_execution_mode(width, height, execution_mode);
    if !transparent {
        let path = Rect::new(0.0, 0.0, width as f64, height as f64).to_path(0.1);

        ctx.set_paint(palette::css::WHITE.into());
        ctx.fill_path(&path.into());
    }

    ctx
}

pub fn render_pixmap(ctx: &RenderContext) -> Pixmap {
    let mut pixmap = Pixmap::new(ctx.width(), ctx.height());
    ctx.render_to_pixmap(&mut pixmap);

    pixmap
}

pub fn check_ref(ctx: &RenderContext, name: &str) {
    let mut pixmap = render_pixmap(ctx);
    pixmap.unpremultiply();

    let encoded_image = {
        let mut out = vec![];
        let mut encoder = png::Encoder::new(&mut out, ctx.width() as u32, ctx.height() as u32);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixmap.data()).unwrap();
        writer.finish().unwrap();

        out
    };

    let ref_path = REFS_PATH.join(format!("{}.png", name));

    let write_ref_image = || {
        let optimized =
            oxipng::optimize_from_memory(&encoded_image, &oxipng::Options::max_compression())
                .unwrap();
        std::fs::write(&ref_path, optimized).unwrap();
    };

    if !ref_path.exists() {
        write_ref_image();
        panic!("new reference image was created");
    }

    let ref_image = load_from_memory(&std::fs::read(&ref_path).unwrap())
        .unwrap()
        .into_rgba8();
    let actual = load_from_memory(&encoded_image).unwrap().into_rgba8();

    let diff_image = get_diff(&ref_image, &actual);

    if let Some(diff_image) = diff_image {
        if REPLACE {
            write_ref_image();
            panic!("test was replaced");
        }

        let diff_path = DIFFS_PATH.join(format!("{}.png", name));
        diff_image
            .save_with_format(&diff_path, image::ImageFormat::Png)
            .unwrap();

        panic!("test didnt match reference image");
    }
}

fn get_diff(expected_image: &RgbaImage, actual_image: &RgbaImage) -> Option<RgbaImage> {
    let width = max(expected_image.width(), actual_image.width());
    let height = max(expected_image.height(), actual_image.height());

    let mut diff_image = RgbaImage::new(width * 3, height);

    let mut pixel_diff = 0;

    for x in 0..width {
        for y in 0..height {
            let actual_pixel = actual_image.get_pixel_checked(x, y);
            let expected_pixel = expected_image.get_pixel_checked(x, y);

            match (actual_pixel, expected_pixel) {
                (Some(actual), Some(expected)) => {
                    diff_image.put_pixel(x, y, *expected);
                    diff_image.put_pixel(x + 2 * width, y, *actual);
                    if is_pix_diff(expected, actual) {
                        pixel_diff += 1;
                        diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                    } else {
                        diff_image.put_pixel(x + width, y, Rgba([0, 0, 0, 255]))
                    }
                }
                (Some(actual), None) => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x + 2 * width, y, *actual);
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
                (None, Some(expected)) => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x, y, *expected);
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
                _ => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
            }
        }
    }

    if pixel_diff > 0 {
        Some(diff_image)
    } else {
        None
    }
}

fn is_pix_diff(pixel1: &Rgba<u8>, pixel2: &Rgba<u8>) -> bool {
    if pixel1.0[3] == 0 && pixel2.0[3] == 0 {
        return false;
    }

    pixel1.0[0] != pixel2.0[0]
        || pixel1.0[1] != pixel2.0[1]
        || pixel1.0[2] != pixel2.0[2]
        || pixel1.0[3] != pixel2.0[3]
}
