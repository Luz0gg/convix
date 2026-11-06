//! Safety limits against "decompression bombs": a small file that claims huge dimensions
//! and would make the app reserve tens of GB. Running out of memory is not a panic — it
//! kills the whole app — so sizes are checked *before* allocating.

use crate::{Error, Result};

/// Largest image we decode or create: 268 Mpx (16384 × 16384), ~1 GB in RGBA.
/// Far above any camera or scan (a 100 Mpx medium format is 11600 × 8700).
pub const MAX_PIXELS: u64 = 16_384 * 16_384;

/// Largest side accepted, whatever the other one is.
pub const MAX_SIDE: u32 = 65_535;

/// Memory budget for all the frames of an animation together (they are kept decoded).
pub const MAX_ANIMATION_BYTES: u64 = 2 << 30;

/// Largest decompressed stream we accept from a PDF (images, page contents).
pub const MAX_STREAM_BYTES: u64 = 512 << 20;

/// Fails with a clear message if a `width × height` image is too big to handle safely.
pub fn check_image_size(width: u32, height: u32) -> Result<()> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 {
        return Err(Error::Failed("image vide (0 pixel)".into()));
    }
    if width > MAX_SIDE || height > MAX_SIDE || pixels > MAX_PIXELS {
        return Err(Error::Failed(format!(
            "image trop grande ({width} × {height} pixels, maximum {} millions de pixels)",
            MAX_PIXELS / 1_000_000
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert!(check_image_size(6000, 4000).is_ok());
        assert!(check_image_size(16_384, 16_384).is_ok());
        assert!(check_image_size(1, 10_000_000).is_err());
        assert!(check_image_size(20_000, 20_000).is_err());
        assert!(check_image_size(0, 10).is_err());
    }
}
