//! Reading images: raster formats via `image`, SVG via `resvg`.

use convertio_core::{Error, Format, Result};
use image::{DynamicImage, ImageDecoder, ImageReader, Limits, RgbaImage};
use resvg::{tiny_skia, usvg};
use std::{
    path::Path,
    sync::{Arc, OnceLock},
};

/// A decoded image plus what we want to carry over to the output.
pub struct Decoded {
    pub image: DynamicImage,
    /// Embedded color profile (Display P3 photos…), re-embedded when the output supports it.
    pub icc: Option<Vec<u8>>,
}

pub fn decode(path: &Path, format: Format, svg_width: Option<u32>) -> Result<Decoded> {
    match format {
        Format::Svg => Ok(Decoded { image: DynamicImage::ImageRgba8(render_svg(path, svg_width)?), icc: None }),
        _ => decode_raster(path),
    }
}

fn decode_raster(path: &Path) -> Result<Decoded> {
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(4 << 30); // 4 GB: big panoramas / scans are legitimate
    reader.limits(limits);

    let mut decoder = reader.into_decoder().map_err(img_err)?;
    // Phone photos are stored sideways with an EXIF "rotate" flag: apply it,
    // otherwise the converted file comes out rotated.
    let orientation = decoder.orientation().map_err(img_err)?;
    let icc = decoder.icc_profile().ok().flatten();
    let mut image = DynamicImage::from_decoder(decoder).map_err(img_err)?;
    image.apply_orientation(orientation);
    Ok(Decoded { image, icc })
}

fn fontdb() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        Arc::new(db)
    })
    .clone()
}

/// Rasterize an SVG at `width` px (keeping its ratio), or at its own size.
fn render_svg(path: &Path, width: Option<u32>) -> Result<RgbaImage> {
    let data = std::fs::read(path)?;
    let options = usvg::Options {
        resources_dir: path.parent().map(Path::to_path_buf),
        fontdb: fontdb(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_data(&data, &options).map_err(|e| Error::Failed(format!("SVG: {e}")))?;

    let size = tree.size();
    let scale = match width {
        Some(w) if w > 0 => w as f32 / size.width(),
        _ => 1.0,
    };
    let (w, h) = ((size.width() * scale).round().max(1.0) as u32, (size.height() * scale).round().max(1.0) as u32);
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or_else(|| Error::Failed("SVG: size too large".into()))?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());

    // tiny-skia stores premultiplied alpha; image expects straight alpha.
    let raw: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    RgbaImage::from_raw(w, h, raw).ok_or_else(|| Error::Failed("SVG: bad buffer".into()))
}

pub fn img_err(e: image::ImageError) -> Error {
    match e {
        image::ImageError::IoError(io) => Error::Io(io),
        image::ImageError::Unsupported(u) => Error::Failed(format!("format non supporté : {u}")),
        other => Error::Failed(other.to_string()),
    }
}
