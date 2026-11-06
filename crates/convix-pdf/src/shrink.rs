//! "Smaller" PDF compression: photos inside the PDF are downscaled and re-encoded as JPEG.
//! Only images we fully understand are touched (8-bit RGB / gray); everything else is kept.

use convix_image::{
    EncodeSettings, encode,
    image::{self, DynamicImage, GrayImage, RgbImage, imageops::FilterType},
};
use convix_core::{CancelToken, Format, Result};
use lopdf::{Document, Object, Stream};

pub struct ShrinkSettings {
    /// Longest side, in pixels, after downscaling.
    pub max_side: u32,
    pub quality: f32,
}

fn name_of(obj: Option<&Object>) -> Option<&[u8]> {
    match obj? {
        Object::Name(n) => Some(n),
        Object::Array(a) if a.len() == 1 => a[0].as_name().ok(),
        _ => None,
    }
}

/// Number of color components if the color space is one we can re-encode as JPEG.
fn components(doc: &Document, cs: Option<&Object>) -> Option<u8> {
    let cs = match cs? {
        Object::Reference(id) => doc.get_object(*id).ok()?,
        other => other,
    };
    match cs {
        Object::Name(n) if n == b"DeviceRGB" => Some(3),
        Object::Name(n) if n == b"DeviceGray" => Some(1),
        // [/ICCBased <stream>]: keep the profile, the pixel layout is plain RGB/gray.
        Object::Array(a) if a.len() == 2 && a[0].as_name().ok() == Some(b"ICCBased") => {
            let id = a[1].as_reference().ok()?;
            let n = doc.get_object(id).ok()?.as_stream().ok()?.dict.get(b"N").ok()?.as_i64().ok()?;
            matches!(n, 1 | 3).then_some(n as u8)
        }
        _ => None,
    }
}

fn decode_pixels(doc: &Document, stream: &Stream) -> Option<(DynamicImage, u8)> {
    let d = &stream.dict;
    let width = d.get(b"Width").ok()?.as_i64().ok()? as u32;
    let height = d.get(b"Height").ok()?.as_i64().ok()? as u32;
    if d.has(b"Decode") || d.has(b"ImageMask") || d.get(b"BitsPerComponent").ok()?.as_i64().ok()? != 8 {
        return None;
    }
    let comps = components(doc, d.get(b"ColorSpace").ok())?;
    match name_of(d.get(b"Filter").ok())? {
        b"DCTDecode" => {
            let img = image::load_from_memory_with_format(&stream.content, image::ImageFormat::Jpeg).ok()?;
            // CMYK JPEGs are converted by the decoder: leave those alone.
            (img.color().channel_count() == comps).then_some((img, comps))
        }
        b"FlateDecode" => {
            let raw = stream.decompressed_content().ok()?;
            match comps {
                3 => RgbImage::from_raw(width, height, raw).map(|i| (DynamicImage::ImageRgb8(i), 3)),
                _ => GrayImage::from_raw(width, height, raw).map(|i| (DynamicImage::ImageLuma8(i), 1)),
            }
        }
        _ => None,
    }
}

/// Re-encode the document's images. Returns how many were replaced.
pub fn shrink_images(doc: &mut Document, s: &ShrinkSettings, cancel: &CancelToken) -> Result<usize> {
    let ids: Vec<_> = doc
        .objects
        .iter()
        .filter(|(_, o)| {
            o.as_stream().is_ok_and(|st| st.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image"))
        })
        .map(|(id, _)| *id)
        .collect();

    let mut replaced = 0;
    for id in ids {
        cancel.check()?;
        let Ok(stream) = doc.get_object(id).and_then(Object::as_stream) else { continue };
        let Some((img, comps)) = decode_pixels(doc, stream) else { continue };
        let was_jpeg = name_of(stream.dict.get(b"Filter").ok()) == Some(b"DCTDecode");
        let original = stream.content.len();

        let img = if img.width().max(img.height()) > s.max_side {
            img.resize(s.max_side, s.max_side, FilterType::Lanczos3)
        } else {
            img
        };
        let settings = EncodeSettings { quality: s.quality, ..EncodeSettings::for_format(Format::Jpeg) };
        let Ok(jpeg) = encode(&img, None, Format::Jpeg, &settings) else { continue };

        // JPEG must clearly win; for lossless (Flate) images, by a lot, to spare screenshots and diagrams.
        let threshold = if was_jpeg { original * 9 / 10 } else { original / 2 };
        if jpeg.len() >= threshold {
            continue;
        }
        let (w, h) = (img.width() as i64, img.height() as i64);
        if let Ok(Object::Stream(stream)) = doc.get_object_mut(id) {
            stream.dict.set("Filter", "DCTDecode");
            stream.dict.remove(b"DecodeParms");
            stream.dict.set("Width", w);
            stream.dict.set("Height", h);
            if comps == 1 {
                stream.dict.set("ColorSpace", "DeviceRGB"); // our JPEG encoder writes RGB
            }
            stream.set_content(jpeg);
            stream.allows_compression = false; // already compressed
            replaced += 1;
        }
    }
    Ok(replaced)
}
