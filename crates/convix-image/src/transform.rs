//! Pixel-level operations: resizing and palette reduction.

use convix_core::{Error, Result, limits};
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use rayon::prelude::*;

/// Resize. With `keep_ratio` and both sides given, the image fits inside the box.
/// With a single side, the other one follows the original ratio.
pub fn resize(img: &DynamicImage, width: Option<u32>, height: Option<u32>, keep_ratio: bool) -> Result<DynamicImage> {
    let (w0, h0) = img.dimensions();
    let scaled = |num: u32, a: u32, b: u32| ((num as f64 * a as f64 / b as f64).round() as u32).max(1);
    let (w, h) = match (width.filter(|&w| w > 0), height.filter(|&h| h > 0)) {
        (None, None) => {
            return Err(Error::InvalidOption {
                name: "width".into(),
                reason: "indique au moins une largeur ou une hauteur".into(),
            });
        }
        (Some(w), None) => (w, scaled(h0, w, w0)),
        (None, Some(h)) => (scaled(w0, h, h0), h),
        (Some(w), Some(h)) if keep_ratio => {
            let (fw, fh) = scaled_fit(w0, h0, w, h); // fits inside the box
            limits::check_image_size(fw, fh)?;
            return Ok(img.resize(w, h, FilterType::Lanczos3));
        }
        (Some(w), Some(h)) => (w, h),
    };
    limits::check_image_size(w, h)?;
    Ok(img.resize_exact(w, h, FilterType::Lanczos3))
}

/// Size of `(w0, h0)` fitted inside `(w, h)`, as `DynamicImage::resize` does.
fn scaled_fit(w0: u32, h0: u32, w: u32, h: u32) -> (u32, u32) {
    let ratio = f64::min(f64::from(w) / f64::from(w0), f64::from(h) / f64::from(h0));
    (
        (f64::from(w0) * ratio).round().max(1.0) as u32,
        (f64::from(h0) * ratio).round().max(1.0) as u32,
    )
}

/// Reduce to `colors` colors (NeuQuant). Combined with oxipng, which then stores the
/// result as a palette PNG, this typically cuts PNG size by 60–80 %.
pub fn quantize(img: &DynamicImage, colors: usize) -> DynamicImage {
    let mut rgba = img.to_rgba8();
    let nq = color_quant::NeuQuant::new(10, colors.clamp(2, 256), rgba.as_raw());
    let map = nq.color_map_rgba();
    rgba.par_chunks_mut(4).for_each(|px| {
        let i = nq.index_of(px) * 4;
        px.copy_from_slice(&map[i..i + 4]);
    });
    DynamicImage::ImageRgba8(rgba)
}
