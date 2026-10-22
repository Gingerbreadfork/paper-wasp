//! Extracts positioned text from a page by interpreting it with a glyph-only device.

use crate::protocol::TextLine;
use hayro::hayro_interpret::font::{Glyph, GlyphRun};
use hayro::hayro_interpret::hayro_syntax::Pdf;
use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_interpret::{
    interpret_page, BlendMode, ClipPath, Context, Device, DrawMode, DrawProps, Image,
    ImageDrawProps, InterpreterCache, InterpreterSettings, SoftMask,
};
use hayro::hayro_interpret::hayro_cmap::BfString;
use hayro::kurbo::{Affine, BezPath, Rect};

struct GlyphBox {
    text: String,
    rect: Rect,
    size: f64,
}

#[derive(Default)]
struct TextDevice {
    boxes: Vec<GlyphBox>,
}

impl<'a> Device<'a> for TextDevice {
    fn draw_path(&mut self, _: &BezPath, _: DrawProps<'a>, _: &DrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_image(&mut self, _: Image<'a, '_>, _: ImageDrawProps<'a>) {}
    fn pop_clip(&mut self) {}
    fn pop_transparency_group(&mut self) {}

    fn draw_glyph_run(&mut self, run: &GlyphRun<'_, 'a>, props: DrawProps<'a>, _: &DrawMode) {
        for glyph in run.glyphs() {
            let Glyph::Outline(outline) = &**glyph else { continue };
            let Some(unicode) = outline.as_unicode() else { continue };
            let text = match unicode {
                BfString::Char(c) => c.to_string(),
                BfString::String(s) => s,
            };
            if text.is_empty() {
                continue;
            }
            let advance = outline.advance_width().unwrap_or(500.0).max(1.0) as f64;
            let transform: Affine = props.transform * glyph.transform();
            let rect = transform.transform_rect_bbox(Rect::new(0.0, -220.0, advance, 780.0));
            let c = transform.as_coeffs();
            let size = ((c[1] * c[1] + c[3] * c[3]).sqrt() * 1000.0).abs();
            if !rect.is_finite() || size <= 0.0 {
                continue;
            }
            self.boxes.push(GlyphBox { text, rect, size });
        }
    }
}

pub fn page_text<'a>(
    pdf: &'a Pdf,
    cache: &InterpreterCache<'a>,
    settings: &InterpreterSettings,
    index: usize,
) -> Result<Vec<TextLine>, String> {
    let pages = pdf.pages();
    let page = pages.get(index).ok_or_else(|| format!("page {index} is out of range"))?;
    let (w, h) = page.render_dimensions();
    let transform = page.initial_transform(true).to_kurbo();
    let mut ctx = Context::new(
        transform,
        Rect::new(0.0, 0.0, w as f64, h as f64),
        cache,
        page.xref(),
        settings.clone(),
    );
    let mut device = TextDevice::default();
    interpret_page(page, &mut ctx, &mut device);
    Ok(group_lines(device.boxes))
}

fn group_lines(mut boxes: Vec<GlyphBox>) -> Vec<TextLine> {
    if boxes.is_empty() {
        return Vec::new();
    }
    boxes.sort_by(|a, b| {
        let ay = a.rect.center().y;
        let by = b.rect.center().y;
        ay.partial_cmp(&by).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut groups: Vec<Vec<GlyphBox>> = Vec::new();
    let mut current: Vec<GlyphBox> = Vec::new();
    let mut cur_y: f64 = 0.0;
    let mut cur_h: f64 = 0.0;
    for b in boxes {
        let cy = b.rect.center().y;
        let h = b.rect.height().max(1.0);
        if current.is_empty() || (cy - cur_y).abs() <= 0.45 * cur_h.min(h) {
            if current.is_empty() {
                cur_y = cy;
                cur_h = h;
            } else {
                let n = current.len() as f64;
                cur_y = (cur_y * n + cy) / (n + 1.0);
                cur_h = cur_h.max(h);
            }
            current.push(b);
        } else {
            groups.push(std::mem::take(&mut current));
            current.push(b);
            cur_y = cy;
            cur_h = h;
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }

    let mut lines = Vec::with_capacity(groups.len());
    for mut g in groups {
        g.sort_by(|a, b| a.rect.x0.partial_cmp(&b.rect.x0).unwrap_or(std::cmp::Ordering::Equal));
        let mut text = String::new();
        let mut xs: Vec<f32> = Vec::new();
        let mut bbox: Option<Rect> = None;
        let mut last_right = f64::NEG_INFINITY;
        let mut last_size: f64 = 0.0;
        for b in g {
            let r = b.rect;
            if r.width() <= 0.0 {
                continue;
            }
            if last_right.is_finite() && r.x0 < last_right - 0.6 * b.size {
                continue;
            }
            let gap = r.x0 - last_right;
            if last_right.is_finite() && gap > 0.22 * last_size.max(b.size) && !text.ends_with(' ') {
                text.push(' ');
                xs.push(last_right as f32);
            }
            let n = b.text.chars().count().max(1);
            let step = r.width() / n as f64;
            for i in 0..n {
                xs.push((r.x0 + step * i as f64) as f32);
            }
            text.push_str(&b.text);
            last_right = r.x1;
            last_size = b.size;
            bbox = Some(match bbox {
                Some(bb) => bb.union(r),
                None => r,
            });
        }
        let Some(bb) = bbox else { continue };
        if text.trim().is_empty() {
            continue;
        }
        xs.push(bb.x1 as f32);
        lines.push(TextLine {
            text,
            x: bb.x0 as f32,
            y: bb.y0 as f32,
            w: bb.width() as f32,
            h: bb.height() as f32,
            xs,
        });
    }
    lines
}
