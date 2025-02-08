use cpu_sparse::paint::Paint;
use cpu_sparse::strip::{Strip};
use cpu_sparse::svg::{render_tree, SVGContext};
use cpu_sparse::tiling::{FlatLine, Point, Tile, TILE_HEIGHT, TILE_WIDTH};
use cpu_sparse::wide_tile::{Cmd, WideTile, STRIP_HEIGHT};
use cpu_sparse::{FillRule, RenderContext};
use peniko::color::palette;
use peniko::kurbo::{Affine, BezPath, Stroke};
use rand::Rng;
use std::collections::HashSet;
use svg::node::element::path::Data;
use svg::node::element::{Circle, Path, Rectangle};
use svg::{Document, Node};

const WIDTH: usize = 50;
const HEIGHT: usize = 50;

fn main() {
    let mut document = Document::new().set("viewBox", (-10, -10, WIDTH + 20, HEIGHT + 20));

    let ctx = ctx();

    // let svg = std::fs::read_to_string("svgs/gs.svg").expect("error reading file");
    // let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    // let mut ctx = RenderContext::new(WIDTH, HEIGHT);
    // let mut sctx = SVGContext::new();
    // render_tree(&mut ctx, &mut sctx, &tree);

    draw_grid(&mut document);
    draw_line_segments(&mut document, &ctx.line_buf);
    draw_tile_areas(&mut document, &ctx.tile_buf);
    draw_tile_intersections(&mut document, &ctx.tile_buf);
    draw_strips(&mut document, &ctx.strip_buf, &ctx.alphas);
    draw_wide_tiles(&mut document, &ctx.tiles, &ctx.alphas);

    svg::save("target/out.svg", &document).unwrap();
}

fn ctx() -> RenderContext {
    let mut ctx = RenderContext::new(WIDTH, HEIGHT);

    let path = {
        let mut path = BezPath::new();
        path.move_to((5.0, 0.0));
        path.line_to((15.5, 12.5));
        path.line_to((3.5, 23.0));
        path.line_to((-7.5, 11.5));
        path.close_path();

        path
    };

    ctx.transform(Affine::translate((1.0, 0.0)));
    ctx.fill_path(&path.into(), FillRule::EvenOdd, palette::css::LIME.into());
    // let stroke = Stroke::new(3.0);
    // ctx.stroke(&piet_path, &stroke, palette::css::DARK_BLUE.into());

    ctx
}

fn draw_wide_tiles(document: &mut Document, wide_tiles: &[WideTile], alphas: &[u32]) {
    for (t_i, tile) in wide_tiles.iter().enumerate() {
        for cmd in &tile.cmds {
            match cmd {
                Cmd::Fill(f) => {
                    for i in 0..f.width {
                        let Paint::Solid(c) = f.paint else { continue };
                        let color = c.to_rgba8();

                        for h in 0..STRIP_HEIGHT {
                            let rect = Rectangle::new()
                                .set("x", f.x + i)
                                .set("y", t_i * STRIP_HEIGHT + h)
                                .set("width", 1)
                                .set("height", 1)
                                .set(
                                    "fill",
                                    format!("rgb({}, {}, {})", color.r, color.g, color.b),
                                )
                                .set("fill-opacity", color.a as f32 / 255.0);

                            document.append(rect);
                        }
                    }
                }
                Cmd::Strip(s) => {
                    for i in 0..s.width {
                        let alpha = alphas[s.alpha_ix + i as usize];
                        let entries = alpha.to_le_bytes();
                        let Paint::Solid(c) = s.paint else { continue };
                        let color = c.to_rgba8();

                        for h in 0..STRIP_HEIGHT {
                            let rect = Rectangle::new()
                                .set("x", s.x + i)
                                .set("y", t_i * STRIP_HEIGHT + h)
                                .set("width", 1)
                                .set("height", 1)
                                .set(
                                    "fill",
                                    format!("rgb({}, {}, {})", color.r, color.g, color.b),
                                )
                                .set(
                                    "fill-opacity",
                                    (entries[h] as f32 / 255.0) * (color.a as f32 / 255.0),
                                );

                            document.append(rect);
                        }
                    }
                }
            }
        }
    }
}

fn draw_tile_areas(document: &mut Document, tiles: &[Tile]) {
    let mut seen = HashSet::new();

    for tile in tiles {
        // Draw the points
        let x = tile.x() * TILE_WIDTH as i32;
        let y = tile.y() * TILE_HEIGHT as i32;

        if seen.contains(&(x, y)) {
            continue;
        }

        let color = if tile.x() == 95 && tile.y() == 86 {
            "red"
        } else {
            "darkblue"
        };

        let rect = Rectangle::new()
            .set("x", x)
            .set("y", y)
            .set("width", TILE_WIDTH)
            .set("height", TILE_HEIGHT)
            .set("fill", color)
            .set("stroke", color)
            .set("stroke-opacity", 0.6)
            .set("stroke-width", 0.2)
            .set("fill-opacity", 0.1);

        document.append(rect);

        seen.insert((x, y));
    }
}

fn draw_strips(document: &mut Document, strips: &[Strip], alphas: &[u32]) {
    for i in 0..strips.len() {
        let strip = &strips[i];
        let x = strip.x();
        let y = strip.strip_y();

        let end = strips
            .get(i + 1)
            .map(|s| s.col)
            .unwrap_or(alphas.len() as u32);

        let width = end - strip.col;

        let color = if strip.winding != 0 {
            "red"
        } else {
            "limegreen"
        };

        let rect = Rectangle::new()
            .set("x", x)
            .set("y", y * STRIP_HEIGHT as u32)
            .set("width", width)
            .set("height", 1 * STRIP_HEIGHT)
            .set("stroke", color)
            .set("fill", color)
            .set("fill-opacity", 0.4)
            .set("stroke-opacity", 0.6)
            .set("stroke-width", 0.2);

        document.append(rect);
    }

    for i in 0..strips.len() {
        let strip = &strips[i];
        // Draw the points
        let x = strip.x();
        let y = strip.strip_y();

        let end = strips
            .get(i + 1)
            .map(|s| s.col)
            .unwrap_or(alphas.len() as u32);

        let width = end - strip.col;

        let color = if strip.winding != 0 {
            "red"
        } else {
            "limegreen"
        };

        for i in 0..width {
            let alpha = alphas[(i + strip.col) as usize];
            let entries = alpha.to_le_bytes();

            for h in 0..STRIP_HEIGHT {
                let rect = Rectangle::new()
                    .set("x", x + i as i32)
                    .set("y", y * STRIP_HEIGHT as u32 + h as u32)
                    .set("width", 1)
                    .set("height", 1)
                    .set("fill", color)
                    .set("fill-opacity", entries[h] as f32 / 255.0);

                document.append(rect);
            }
        }
    }
}

fn draw_tile_intersections(document: &mut Document, tiles: &[Tile]) {
    for tile in tiles {
        // Draw the points
        let x = tile.x() * TILE_WIDTH as i32;
        let y = tile.y() * TILE_HEIGHT as i32;

        let p0 = tile.p0().unpack();
        let p1 = tile.p1().unpack();
        for p in [(p0, -0.05, "darkgreen"), (p1, 0.05, "purple")] {
            let circle = Circle::new()
                .set("cx", x as f32 + p.0.x + p.1)
                .set("cy", y as f32 + p.0.y)
                .set("r", 0.3)
                .set("fill", p.2)
                .set("fill-opacity", 0.5);

            document.append(circle);
        }
    }
}

fn draw_line_segments(document: &mut Document, line_buf: &[FlatLine]) {
    let mut data = Data::new();

    let mut last = None;

    for line in line_buf {
        let first = (line.p0.x, line.p0.y);
        let second = (line.p1.x, line.p1.y);

        if Some(first) != last {
            data = data.move_to(first)
        }

        data = data.line_to(second);

        last = Some(second);
    }

    let border = Path::new()
        .set("stroke-width", 0.1)
        .set("stroke", "green")
        .set("fill", "yellow")
        .set("fill-opacity", 0.1)
        .set("d", data);

    document.append(border);
}

fn draw_grid(document: &mut Document) {
    let border_data = Data::new()
        .move_to((0, 0))
        .line_to((WIDTH, 0))
        .line_to((WIDTH, HEIGHT))
        .line_to((0, HEIGHT))
        .close();

    let border = Path::new()
        .set("stroke-width", 0.2)
        .set("fill", "none")
        .set("vectorEffect", "non-scaling-stroke")
        .set("stroke", "black")
        .set("d", border_data);

    let grid_line = |data: Data| {
        Path::new()
            .set("stroke", "grey")
            .set("stroke-opacity", 0.3)
            .set("stroke-width", 0.1)
            .set("vectorEffect", "non-scaling-stroke")
            .set("d", data)
    };

    for i in 1..HEIGHT {
        let data = Data::new().move_to((0, i)).line_to((WIDTH, i));

        document.append(grid_line(data));
    }

    for i in 1..WIDTH {
        let data = Data::new().move_to((i, 0)).line_to((i, HEIGHT));

        document.append(grid_line(data));
    }

    document.append(border);
}
