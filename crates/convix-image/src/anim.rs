//! Animated images: GIF, animated WebP and APNG in; GIF and animated WebP out.
//! A still file returns `None` and goes through the normal single-image path.

use crate::decode::img_err;
use convix_core::{Error, Format, Result, limits};
use image::{
    AnimationDecoder, Frame, Frames, ImageDecoder,
    codecs::{
        gif::{GifDecoder, GifEncoder, Repeat},
        png::PngDecoder,
        webp::WebPDecoder,
    },
};
use std::{fs::File, io::BufReader, path::Path};

/// All frames, composited to full canvas size, if the file has more than one.
pub fn decode_frames(path: &Path, format: Format) -> Result<Option<Vec<Frame>>> {
    let reader = BufReader::new(File::open(path)?);
    let frames = match format {
        Format::Gif => {
            let d = GifDecoder::new(reader).map_err(img_err)?;
            let (w, h) = d.dimensions();
            collect_limited(d.into_frames(), w, h)?
        }
        Format::Webp => {
            let d = WebPDecoder::new(reader).map_err(img_err)?;
            if !d.has_animation() {
                return Ok(None);
            }
            let (w, h) = d.dimensions();
            collect_limited(d.into_frames(), w, h)?
        }
        Format::Png => {
            let d = PngDecoder::new(reader).map_err(img_err)?;
            if !d.is_apng().map_err(img_err)? {
                return Ok(None);
            }
            let (w, h) = d.dimensions();
            collect_limited(d.apng().map_err(img_err)?.into_frames(), w, h)?
        }
        _ => return Ok(None),
    };
    Ok((frames.len() > 1).then_some(frames))
}

/// Every frame is kept at full canvas size: check the canvas, then stop before the frames
/// together exceed the memory budget (a tiny GIF can declare 65535 × 65535 × 1000 frames).
fn collect_limited(frames: Frames<'_>, w: u32, h: u32) -> Result<Vec<Frame>> {
    limits::check_image_size(w, h)?;
    let per_frame = u64::from(w) * u64::from(h) * 4;
    let max_frames = (limits::MAX_ANIMATION_BYTES / per_frame).max(1) as usize;
    let mut out = Vec::new();
    for frame in frames {
        if out.len() >= max_frames {
            return Err(Error::Failed(format!(
                "animation trop lourde (plus de {max_frames} images de {w} × {h})"
            )));
        }
        out.push(frame.map_err(img_err)?);
    }
    Ok(out)
}

fn delay_ms(frame: &Frame) -> i32 {
    let (num, den) = frame.delay().numer_denom_ms();
    (num / den.max(1)).min(i32::MAX as u32) as i32
}

pub fn encode_gif(frames: Vec<Frame>) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut enc = GifEncoder::new_with_speed(&mut buf, 10);
        enc.set_repeat(Repeat::Infinite).map_err(img_err)?;
        enc.encode_frames(frames).map_err(img_err)?;
    }
    Ok(buf)
}

pub fn encode_webp(frames: &[Frame], quality: f32, lossless: bool) -> Result<Vec<u8>> {
    let (w, h) = frames[0].buffer().dimensions();
    if w > 16_383 || h > 16_383 {
        return Err(Error::Failed(format!("WebP : {w} × {h} dépasse la limite du format (16383 pixels)")));
    }
    let mut config = webp::WebPConfig::new().map_err(|_| Error::Failed("WebP : configuration impossible".into()))?;
    config.lossless = lossless as i32;
    config.alpha_compression = (!lossless) as i32;
    config.quality = quality.clamp(1.0, 100.0);

    let mut enc = webp::AnimEncoder::new(w, h, &config);
    enc.set_loop_count(0); // loop forever
    let mut timestamp = 0;
    for f in frames {
        enc.add_frame(webp::AnimFrame::from_rgba(f.buffer().as_raw(), w, h, timestamp));
        timestamp = timestamp.saturating_add(delay_ms(f));
    }
    enc.try_encode().map(|m| m.to_vec()).map_err(|e| Error::Failed(format!("WebP animé : {e:?}")))
}
