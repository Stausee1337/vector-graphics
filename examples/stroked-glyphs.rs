use std::f32;

use skrifa::{FontRef, MetadataProvider, instance::Location, outline::{DrawSettings, OutlinePen}, prelude::Size};

use vector_graphics::{
    affine::Affine, path::{Path, PathElement, transform_path}, stroke::{Join, Stroke, expand_stroke}, vec::Vec2
};


const JBM_REGULAR: &'static [u8] = include_bytes!("../assets/JetBrainsMono-2.304/JetBrainsMono-Regular.ttf");

pub struct SkrifaOutlinePen<'a> {
    path: &'a mut Path
}

impl<'a> SkrifaOutlinePen<'a> {
    fn new(path: &'a mut Path) -> Self {
        SkrifaOutlinePen { path }
    }
}

impl<'a> OutlinePen for SkrifaOutlinePen<'a> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.push(PathElement::MoveTo(Vec2::new(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.push(PathElement::LineTo(Vec2::new(x, y)));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.push(PathElement::QuadTo(Vec2::new(cx0, cy0), Vec2::new(x, y)));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path.push(PathElement::CurveTo(Vec2::new(cx0, cy0), Vec2::new(cx1, cy1), Vec2::new(x, y))); 
    }

    fn close(&mut self) {
        self.path.push(PathElement::Close); 
    }
}

fn get_glyphs(text: &str, font: &FontRef) -> impl Iterator<Item = Path> {
    let charmap = font.charmap();
    let outline_glyphs = font.outline_glyphs();

    let font_metrics = font.metrics(Size::unscaled(), &Location::default());
    let location = Location::default();

    let mut chars = text.chars();
    let mut pen_y = 0.0;

    std::iter::from_fn(move || {
        let Some(char) = chars.next() else {
            return None;
        };

        let glyph_metrics = font.glyph_metrics(Size::unscaled(), &location);

        let glyph_id = charmap.map(char).unwrap();
        let outline_glyph = outline_glyphs.get(glyph_id).unwrap();
        let advance = glyph_metrics.advance_width(glyph_id).unwrap_or_default();

        let mut path = Path::new();
        let mut outline_pen = SkrifaOutlinePen::new(&mut path);
        outline_glyph.draw(DrawSettings::unhinted(Size::unscaled(), &Location::default()), &mut outline_pen).unwrap();

        let res = transform_path(&path, Affine::new([1.0, 0.0, 0.0, -1.0, pen_y, font_metrics.ascent]));
        pen_y += advance;

        Some(res)
    })
}

const TEXT: &'static str = "Vector Graphics";

fn main() {
    let font = FontRef::new(JBM_REGULAR).unwrap();
    let stroke = Stroke { width: 10.0, join: Join::Miter { miter_limit: 4.0 } };

    let mut string = String::new();
    for path in get_glyphs(TEXT, &font) {
        let stroked = expand_stroke(&path, &stroke);
        stroked.as_svg(&mut string).unwrap();
    }

    // TOOD: maybe output as svg, not one giant string of svg path commands
    println!("{string}");
}

