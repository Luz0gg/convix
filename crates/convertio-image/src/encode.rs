//! Writing images. Each output format uses the best available encoder.

use crate::decode::img_err;
use convertio_core::{Error, Format, Result};
use image::{
    DynamicImage, ExtendedColorType, GenericImageView, ImageEncoder, ImageFormat, ImageResult,
    RgbImage, RgbaImage,
    codecs::{
        gif::GifEncoder,
        ico::{IcoEncoder, IcoFrame},
        png::{CompressionType, FilterType as PngFilter, PngEncoder},
    },
    imageops::{self, FilterType},
};
use std::{borrow::Cow, io::Cursor, panic::AssertUnwindSafe};

/// Encoder parameters, filled from the user's options.
#[derive(Debug, Clone)]
pub struct EncodeSettings {
    /// 1–100, for lossy formats (JPEG, WebP, AVIF).
    pub quality: f32,
    /// WebP only.
    pub lossless: bool,
    /// PNG: oxipng preset (0 = fast … 6 = max), `None` = no optimization pass.
    pub png_optimize: Option<u8>,
    /// ICO: sizes embedded in the icon.
    pub ico_sizes: Vec<u32>,
    /// AVIF: 1 (slow, small) … 10 (fast).
    pub avif_speed: u8,
}

impl EncodeSettings {
    pub fn for_format(to: Format) -> Self {
        Self {
            quality: match to {
                Format::Avif => 75.0,
                _ => 90.0,
            },
            lossless: false,
            png_optimize: Some(2),
            ico_sizes: vec![256, 128, 64, 48, 32, 16],
            avif_speed: 6,
        }
    }
}

pub const WRITABLE: &[Format] = &[
    Format::Png,
    Format::Jpeg,
    Format::Webp,
    Format::Avif,
    Format::Ico,
    Format::Gif,
    Format::Tiff,
    Format::Bmp,
    Format::Tga,
];

pub fn encode(img: &DynamicImage, icc: Option<&[u8]>, to: Format, s: &EncodeSettings) -> Result<Vec<u8>> {
    match to {
        Format::Png => png(img, icc, s.png_optimize),
        Format::Jpeg => jpeg(img, icc, s.quality),
        Format::Webp => webp(img, s.quality, s.lossless),
        Format::Avif => avif(img, s.quality, s.avif_speed),
        Format::Gif => gif(img),
        Format::Ico => ico(img, &s.ico_sizes),
        Format::Bmp => with_image_crate(&to_8bit(img), ImageFormat::Bmp),
        Format::Tga => with_image_crate(&to_8bit(img), ImageFormat::Tga),
        Format::Tiff => with_image_crate(&to_tiff_compatible(img), ImageFormat::Tiff),
        other => Err(Error::Failed(format!("export {} non supporté", other.label()))),
    }
}

/// True when the image has no visible transparency (lets us drop the alpha channel).
pub fn is_opaque(img: &DynamicImage) -> bool {
    if !img.color().has_alpha() {
        return true;
    }
    match img.as_rgba8() {
        Some(b) => b.pixels().all(|p| p[3] == 255),
        None => img.to_rgba8().pixels().all(|p| p[3] == 255),
    }
}

fn to_8bit(img: &DynamicImage) -> DynamicImage {
    if is_opaque(img) { DynamicImage::ImageRgb8(img.to_rgb8()) } else { DynamicImage::ImageRgba8(img.to_rgba8()) }
}

fn to_tiff_compatible(img: &DynamicImage) -> DynamicImage {
    match img {
        DynamicImage::ImageRgb16(_) | DynamicImage::ImageRgba16(_) | DynamicImage::ImageLuma16(_) | DynamicImage::ImageLuma8(_) => img.clone(),
        _ => to_8bit(img),
    }
}

fn with_image_crate(img: &DynamicImage, format: ImageFormat) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, format).map_err(img_err)?;
    Ok(buf.into_inner())
}

// ---------- PNG ----------

fn png(img: &DynamicImage, icc: Option<&[u8]>, optimize: Option<u8>) -> Result<Vec<u8>> {
    // PNG has no float pixels: HDR sources go to 16-bit.
    let img: Cow<DynamicImage> = match img {
        DynamicImage::ImageRgb32F(_) | DynamicImage::ImageRgba32F(_) => Cow::Owned(DynamicImage::ImageRgba16(img.to_rgba16())),
        _ => Cow::Borrowed(img),
    };
    let mut buf = Vec::new();
    let mut enc = PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, PngFilter::Adaptive);
    if let Some(icc) = icc {
        let _ = enc.set_icc_profile(icc.to_vec());
    }
    img.write_with_encoder(enc).map_err(img_err)?;
    match optimize {
        Some(level) => optimize_png(&buf, level),
        None => Ok(buf),
    }
}

/// Lossless re-compression of PNG bytes. Keeps the color profile, drops useless metadata.
pub fn optimize_png(data: &[u8], level: u8) -> Result<Vec<u8>> {
    let mut opts = oxipng::Options::from_preset(level);
    opts.strip = oxipng::StripChunks::Safe;
    oxipng::optimize_from_memory(data, &opts).map_err(|e| Error::Failed(format!("PNG : {e}")))
}

// ---------- JPEG ----------

/// JPEG has no transparency: composite onto white (what people expect, not black).
fn flatten_on_white(img: &DynamicImage) -> RgbImage {
    if is_opaque(img) {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let p = rgba.get_pixel(x, y);
        let a = p[3] as u32;
        let blend = |c: u8| ((c as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
        image::Rgb([blend(p[0]), blend(p[1]), blend(p[2])])
    })
}

fn jpeg(img: &DynamicImage, icc: Option<&[u8]>, quality: f32) -> Result<Vec<u8>> {
    let rgb = flatten_on_white(img);
    let (w, h) = rgb.dimensions();
    // mozjpeg reports libjpeg errors by panicking: contain it.
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| -> std::io::Result<Vec<u8>> {
        let mut c = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
        c.set_size(w as usize, h as usize);
        c.set_quality(quality.clamp(1.0, 100.0));
        c.set_progressive_mode();
        c.set_optimize_scans(true);
        let mut started = c.start_compress(Vec::new())?;
        if let Some(icc) = icc {
            started.write_icc_profile(icc);
        }
        started.write_scanlines(rgb.as_raw())?;
        started.finish()
    }));
    match result {
        Ok(r) => r.map_err(Error::Io),
        Err(_) => Err(Error::Failed("JPEG : l'encodeur a échoué".into())),
    }
}

// ---------- WebP ----------

fn webp(img: &DynamicImage, quality: f32, lossless: bool) -> Result<Vec<u8>> {
    let (w, h) = img.dimensions();
    if w > 16383 || h > 16383 {
        return Err(Error::Failed("WebP : 16 383 px maximum par côté".into()));
    }
    let q = quality.clamp(1.0, 100.0);
    let encoded = if is_opaque(img) {
        let rgb = img.to_rgb8();
        webp::Encoder::from_rgb(rgb.as_raw(), w, h).encode_simple(lossless, q)
    } else {
        let rgba = img.to_rgba8();
        webp::Encoder::from_rgba(rgba.as_raw(), w, h).encode_simple(lossless, q)
    };
    encoded.map(|m| m.to_vec()).map_err(|e| Error::Failed(format!("WebP : {e:?}")))
}

// ---------- AVIF ----------

fn avif(img: &DynamicImage, quality: f32, speed: u8) -> Result<Vec<u8>> {
    use rgb::FromSlice;
    let (w, h) = (img.width() as usize, img.height() as usize);
    let enc = ravif::Encoder::new()
        .with_quality(quality.clamp(1.0, 100.0))
        .with_alpha_quality(quality.clamp(1.0, 100.0))
        .with_speed(speed);
    let encoded = if is_opaque(img) {
        let rgb = img.to_rgb8();
        enc.encode_rgb(ravif::Img::new(rgb.as_raw().as_rgb(), w, h))
    } else {
        let rgba = img.to_rgba8();
        enc.encode_rgba(ravif::Img::new(rgba.as_raw().as_rgba(), w, h))
    };
    encoded.map(|e| e.avif_file).map_err(|e| Error::Failed(format!("AVIF : {e}")))
}

// ---------- GIF ----------

fn gif(img: &DynamicImage) -> Result<Vec<u8>> {
    let (w, h) = img.dimensions();
    if w > 65535 || h > 65535 {
        return Err(Error::Failed("GIF : 65 535 px maximum par côté".into()));
    }
    let rgba = img.to_rgba8();
    let mut buf = Vec::new();
    GifEncoder::new_with_speed(&mut buf, 10)
        .encode(rgba.as_raw(), w, h, ExtendedColorType::Rgba8)
        .map_err(img_err)?;
    Ok(buf)
}

// ---------- ICO ----------

/// Fit `img` in a transparent `size`×`size` square (icons must be square).
fn square(img: &DynamicImage, size: u32) -> RgbaImage {
    let r = img.resize(size, size, FilterType::Lanczos3).to_rgba8();
    let mut canvas = RgbaImage::new(size, size);
    let (x, y) = ((size - r.width()) / 2, (size - r.height()) / 2);
    imageops::overlay(&mut canvas, &r, x as i64, y as i64);
    canvas
}

fn ico(img: &DynamicImage, sizes: &[u32]) -> Result<Vec<u8>> {
    let squares: Vec<RgbaImage> = sizes.iter().map(|&s| square(img, s.clamp(1, 256))).collect();
    let frames = squares
        .iter()
        .map(|s| IcoFrame::as_png(s.as_raw(), s.width(), s.height(), ExtendedColorType::Rgba8))
        .collect::<ImageResult<Vec<_>>>()
        .map_err(img_err)?;
    let mut buf = Vec::new();
    IcoEncoder::new(&mut buf).encode_images(&frames).map_err(img_err)?;
    Ok(buf)
}
