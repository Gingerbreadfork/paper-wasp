//! Rasterizes one page to an opaque RGB PNG.

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings};

const MAX_SIDE: f32 = 10_000.0;

pub fn render_png<'a>(
    pdf: &'a Pdf,
    cache: &RenderCache<'a>,
    settings: &InterpreterSettings,
    index: usize,
    scale: f32,
) -> Result<(u16, u16, Vec<u8>), String> {
    let pages = pdf.pages();
    let page = pages.get(index).ok_or_else(|| format!("page {index} is out of range"))?;
    let (w, h) = page.render_dimensions();
    let mut s = scale.clamp(0.02, 12.0);
    let longest = w.max(h).max(1.0);
    if longest * s > MAX_SIDE {
        s = MAX_SIDE / longest;
    }
    if w * s < 1.0 || h * s < 1.0 {
        return Err("page has no drawable area".into());
    }

    let pixmap = hayro::render(
        page,
        cache,
        settings,
        &RenderSettings::default(),
        &PixmapSettings { x_scale: s, y_scale: s, bg_color: WHITE },
    );
    let (pw, ph) = (pixmap.width(), pixmap.height());
    let rgba = pixmap.data_as_u8_slice();
    let mut rgb = Vec::with_capacity(pw as usize * ph as usize * 3);
    for px in rgba.chunks_exact(4) {
        rgb.extend_from_slice(&px[..3]);
    }

    let mut out = Vec::with_capacity(rgb.len() / 4);
    {
        let mut enc = png::Encoder::new(&mut out, pw as u32, ph as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut writer = enc.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&rgb).map_err(|e| e.to_string())?;
    }
    Ok((pw, ph, out))
}
