//! Images → PDF, one page per image. JPEG photos are embedded as they are (no quality loss,
//! no size growth); other images are stored losslessly.

use convix_core::{CancelToken, Error, Format, Result, format};
use convix_image::image::{DynamicImage, GenericImageView, ImageDecoder, ImageReader, metadata::Orientation};
use lopdf::{Dictionary, Document, Object, Stream};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    /// Page as big as the image (at 150 DPI).
    Image,
    /// A4 or Letter (in points), turned to match the image, with a margin.
    Paper { width: f32, height: f32, margin: f32 },
}

pub const A4: (f32, f32) = (595.28, 841.89);
pub const LETTER: (f32, f32) = (612.0, 792.0);
/// 1 cm in points.
pub const CM: f32 = 28.35;

/// A picture ready to be placed: its XObject and pixel size.
struct Placed {
    stream: Stream,
    smask: Option<Stream>,
    width: u32,
    height: u32,
}

fn image_dict(width: u32, height: u32, color_space: &str) -> Dictionary {
    let mut d = Dictionary::new();
    d.set("Type", "XObject");
    d.set("Subtype", "Image");
    d.set("Width", width as i64);
    d.set("Height", height as i64);
    d.set("ColorSpace", color_space);
    d.set("BitsPerComponent", 8);
    d
}

/// Number of components of a baseline/progressive JPEG (from its SOF marker).
fn jpeg_components(bytes: &[u8]) -> Option<u8> {
    let mut i = 2;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            return None;
        }
        let marker = bytes[i + 1];
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if matches!(marker, 0xC0..=0xC2) {
            return Some(bytes[i + 9]);
        }
        i += 2 + len;
    }
    None
}

/// A JPEG that can go into the PDF untouched (upright, gray or RGB).
fn raw_jpeg(path: &Path) -> Option<Placed> {
    let bytes = std::fs::read(path).ok()?;
    let comps = jpeg_components(&bytes)?;
    let color_space = match comps {
        1 => "DeviceGray",
        3 => "DeviceRGB",
        _ => return None, // CMYK: decoded and stored as RGB instead
    };
    let (width, height) = {
        let mut decoder = ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format().ok()?.into_decoder().ok()?;
        if decoder.orientation().ok()? != Orientation::NoTransforms {
            return None; // phone photo turned by EXIF: decode, which applies the rotation
        }
        decoder.dimensions()
    };
    let mut dict = image_dict(width, height, color_space);
    dict.set("Filter", "DCTDecode");
    let mut stream = Stream::new(dict, bytes);
    stream.allows_compression = false;
    Some(Placed { stream, smask: None, width, height })
}

fn decoded(path: &Path, fmt: Format) -> Result<Placed> {
    let img = convix_image::decode(path, fmt, None)?.image;
    let (width, height) = img.dimensions();
    let gray = matches!(img, DynamicImage::ImageLuma8(_) | DynamicImage::ImageLuma16(_));
    let stream = if gray {
        Stream::new(image_dict(width, height, "DeviceGray"), img.to_luma8().into_raw())
    } else {
        Stream::new(image_dict(width, height, "DeviceRGB"), img.to_rgb8().into_raw())
    };
    // Transparency → soft mask, so PNG logos keep their transparent background.
    let smask = (!convix_image::is_opaque(&img)).then(|| {
        let alpha: Vec<u8> = img.to_rgba8().pixels().map(|p| p.0[3]).collect();
        Stream::new(image_dict(width, height, "DeviceGray"), alpha)
    });
    Ok(Placed { stream, smask, width, height })
}

fn place(path: &Path) -> Result<Placed> {
    let fmt = format::detect(path)?.ok_or(Error::UnsupportedFormat)?;
    if fmt == Format::Jpeg {
        if let Some(p) = raw_jpeg(path) {
            return Ok(p);
        }
    }
    decoded(path, fmt)
}

pub fn build(images: &[PathBuf], size: PageSize, cancel: &CancelToken) -> Result<Document> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();

    for path in images {
        cancel.check()?;
        let Placed { mut stream, smask, width, height } = place(path)?;
        if let Some(mask) = smask {
            let mask_id = doc.add_object(mask);
            stream.dict.set("SMask", mask_id);
        }
        let image_id = doc.add_object(stream);

        // Page box and where the picture goes (centered, aspect kept).
        let (w, h) = (width as f32, height as f32);
        let (page_w, page_h, draw_w, draw_h) = match size {
            PageSize::Image => {
                let k = 72.0 / 150.0;
                (w * k, h * k, w * k, h * k)
            }
            PageSize::Paper { width: pw, height: ph, margin } => {
                let (pw, ph) = if w > h { (ph.max(pw), ph.min(pw)) } else { (pw.min(ph), pw.max(ph)) };
                let scale = ((pw - 2.0 * margin) / w).min((ph - 2.0 * margin) / h);
                (pw, ph, w * scale, h * scale)
            }
        };
        let (x, y) = ((page_w - draw_w) / 2.0, (page_h - draw_h) / 2.0);
        let content = format!("q {draw_w:.2} 0 0 {draw_h:.2} {x:.2} {y:.2} cm /Im0 Do Q");
        let content_id = doc.add_object(Stream::new(Dictionary::new(), content.into_bytes()));

        let mut xobjects = Dictionary::new();
        xobjects.set("Im0", image_id);
        let mut resources = Dictionary::new();
        resources.set("XObject", xobjects);
        let mut page = Dictionary::new();
        page.set("Type", "Page");
        page.set("Parent", pages_id);
        page.set("MediaBox", vec![0.into(), 0.into(), Object::Real(page_w), Object::Real(page_h)]);
        page.set("Resources", resources);
        page.set("Contents", content_id);
        kids.push(Object::Reference(doc.add_object(page)));
    }

    let mut pages = Dictionary::new();
    pages.set("Type", "Pages");
    pages.set("Count", kids.len() as i64);
    pages.set("Kids", kids);
    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let mut catalog = Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", pages_id);
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", catalog_id);
    doc.compress();
    Ok(doc)
}
