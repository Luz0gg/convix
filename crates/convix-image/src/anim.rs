//! Animated images: GIF, animated WebP and APNG in; GIF and animated WebP out.
//! A still file returns `None` and goes through the normal single-image path.

use crate::decode::img_err;
use convix_core::{Error, Format, Result};
use image::{
    AnimationDecoder, Frame,
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
        Format::Gif => GifDecoder::new(reader).map_err(img_err)?.into_frames().collect_frames(),
        Format::Webp => {
            let d = WebPDecoder::new(reader).map_err(img_err)?;
            if !d.has_animation() {
                return Ok(None);
            }
            d.into_frames().collect_frames()
        }
        Format::Png => {
            let d = PngDecoder::new(reader).map_err(img_err)?;
            if !d.is_apng().map_err(img_err)? {
                return Ok(None);
            }
            d.apng().map_err(img_err)?.into_frames().collect_frames()
        }
        _ => return Ok(None),
    }
    .map_err(img_err)?;
    Ok((frames.len() > 1).then_some(frames))
}

fn delay_ms(frame: &Frame) -> i32 {
    let (num, den) = frame.delay().numer_denom_ms();
    (num / den.max(1)) as i32
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
    let mut config = webp::WebPConfig::new().map_err(|_| Error::Failed("WebP : configuration impossible".into()))?;
    config.lossless = lossless as i32;
    config.alpha_compression = (!lossless) as i32;
    config.quality = quality.clamp(1.0, 100.0);

    let mut enc = webp::AnimEncoder::new(w, h, &config);
    enc.set_loop_count(0); // loop forever
    let mut timestamp = 0;
    for f in frames {
        enc.add_frame(webp::AnimFrame::from_rgba(f.buffer().as_raw(), w, h, timestamp));
        timestamp += delay_ms(f);
    }
    enc.try_encode().map(|m| m.to_vec()).map_err(|e| Error::Failed(format!("WebP animé : {e:?}")))
}
