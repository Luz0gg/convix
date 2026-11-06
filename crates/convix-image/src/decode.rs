//! Reading images: raster formats via `image`, SVG via `resvg`.

use convix_core::{Error, Format, Result, limits};
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
        #[cfg(windows)]
        Format::Heic | Format::Avif => Ok(Decoded { image: crate::wic::decode(path, format)?, icc: None }),
        _ => decode_raster(path),
    }
}

fn decode_raster(path: &Path) -> Result<Decoded> {
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(4 << 30); // 4 GB: big panoramas / scans are legitimate
    limits.max_image_width = Some(limits::MAX_SIDE);
    limits.max_image_height = Some(limits::MAX_SIDE);
    reader.limits(limits);

    let mut decoder = reader.into_decoder().map_err(img_err)?;
    let (w, h) = decoder.dimensions();
    limits::check_image_size(w, h)?;
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
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: local_images_only(),
        },
        ..Default::default()
    };
    let tree = usvg::Tree::from_data(&data, &options).map_err(|e| Error::Failed(format!("SVG: {e}")))?;

    let size = tree.size();
    let scale = match width {
        Some(w) if w > 0 => w as f32 / size.width(),
        _ => 1.0,
    };
    // Checked in f64 before converting: an SVG can declare absurd sizes (1 × 10 000 000).
    let (wf, hf) = (
        (f64::from(size.width()) * f64::from(scale)).round().max(1.0),
        (f64::from(size.height()) * f64::from(scale)).round().max(1.0),
    );
    if !wf.is_finite() || !hf.is_finite() || wf > f64::from(u32::MAX) || hf > f64::from(u32::MAX) {
        return Err(Error::Failed("SVG : dimensions invalides".into()));
    }
    let (w, h) = (wf as u32, hf as u32);
    limits::check_image_size(w, h)?;
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

/// Images linked from an SVG: only files next to it (or in a subfolder).
/// Never absolute paths, `..`, network shares (`\\server\share`) or URLs: opening a
/// network path makes Windows connect to that machine, which a local converter must not do.
fn local_images_only() -> usvg::ImageHrefStringResolverFn<'static> {
    let default = usvg::ImageHrefResolver::default_string_resolver();
    Box::new(move |href: &str, opts: &usvg::Options| {
        is_safe_relative_href(href).then(|| default(href, opts)).flatten()
    })
}

fn is_safe_relative_href(href: &str) -> bool {
    let p = Path::new(href);
    !href.is_empty()
        && !href.contains(':') // drive letters, file:, http:…
        && !href.starts_with(['/', '\\'])
        && p.components().all(|c| matches!(c, std::path::Component::Normal(_) | std::path::Component::CurDir))
}

pub fn img_err(e: image::ImageError) -> Error {
    match e {
        image::ImageError::IoError(io) => Error::Io(io),
        image::ImageError::Unsupported(u) => Error::Failed(format!("format non supporté : {u}")),
        other => Error::Failed(other.to_string()),
    }
}
