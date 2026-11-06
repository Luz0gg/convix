//! Decoding through the Windows Imaging Component (WIC): the same codecs as the Photos app.
//! Gives us HEIC (iPhone photos) and AVIF without shipping any native decoder.
//! Requires the free Store codecs: "HEIF Image Extensions" (+ "HEVC Video Extensions")
//! for HEIC, "AV1 Video Extension" for AVIF. Many Windows 11 PCs already have them.

use convix_core::{Error, Format, Result, limits};
use image::{DynamicImage, RgbaImage, metadata::Orientation};
use std::path::Path;
use windows::{
    Win32::{
        Foundation::GENERIC_READ,
        Graphics::Imaging::*,
        System::{
            Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, StructuredStorage::{PROPVARIANT, PropVariantClear}},
            Variant::VT_UI2,
        },
    },
    core::{HSTRING, w},
};

/// WINCODEC_ERR_COMPONENTNOTFOUND: no codec installed for this format.
const COMPONENT_NOT_FOUND: i32 = 0x88982F50u32 as i32;
/// WINCODEC_ERR_COMPONENTINITIALIZEFAILURE: codec present but unusable — typically the HEIF
/// extension is installed without the HEVC/AV1 video extension it relies on.
const COMPONENT_INIT_FAILURE: i32 = 0x88982F8Bu32 as i32;

fn missing_codec(format: Format) -> Error {
    Error::Failed(match format {
        Format::Heic => "Windows ne sait pas encore lire les HEIC : installe gratuitement « Extensions d'image HEIF » \
                         et « Extensions vidéo HEVC » depuis le Microsoft Store, puis réessaie."
            .into(),
        Format::Avif => "Windows ne sait pas encore lire les AVIF : installe gratuitement « Extension vidéo AV1 » \
                         depuis le Microsoft Store, puis réessaie."
            .into(),
        other => format!("aucun décodeur Windows pour {}", other.label()),
    })
}

pub fn decode(path: &Path, format: Format) -> Result<DynamicImage> {
    let wic_err = |step: &'static str| {
        move |e: windows::core::Error| {
            if e.code().0 == COMPONENT_NOT_FOUND || e.code().0 == COMPONENT_INIT_FAILURE {
                missing_codec(format)
            } else {
                Error::Failed(format!("{} ({step}, 0x{:08X}) : {}", format.label(), e.code().0 as u32, e.message()))
            }
        }
    };

    unsafe {
        // Each worker thread needs COM; "already initialized" results are fine.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let factory: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).map_err(wic_err("factory"))?;
        let decoder = factory
            .CreateDecoderFromFilename(&HSTRING::from(path), None, GENERIC_READ, WICDecodeMetadataCacheOnDemand)
            .map_err(wic_err("open"))?;
        let frame = decoder.GetFrame(0).map_err(wic_err("frame"))?;

        let converter = factory.CreateFormatConverter().map_err(wic_err("converter"))?;
        converter
            .Initialize(&frame, &GUID_WICPixelFormat32bppRGBA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom)
            .map_err(wic_err("pixel format"))?;
        let (mut w, mut h) = (0u32, 0u32);
        converter.GetSize(&mut w, &mut h).map_err(wic_err("size"))?;
        limits::check_image_size(w, h)?;
        let mut buf = vec![0u8; w as usize * h as usize * 4];
        converter.CopyPixels(std::ptr::null(), w * 4, &mut buf).map_err(wic_err("pixels"))?;

        let mut img = DynamicImage::ImageRgba8(RgbaImage::from_raw(w, h, buf).ok_or_else(|| Error::Failed("WIC : tampon invalide".into()))?);
        if let Some(o) = orientation(&frame) {
            img.apply_orientation(o);
        }
        Ok(img)
    }
}

/// Photo orientation as exposed by Windows (EXIF tag or HEIF `irot`, unified).
unsafe fn orientation(frame: &IWICBitmapFrameDecode) -> Option<Orientation> {
    unsafe {
        let reader = frame.GetMetadataQueryReader().ok()?;
        let mut value = PROPVARIANT::default();
        reader.GetMetadataByName(w!("System.Photo.Orientation"), &mut value).ok()?;
        let inner = &value.Anonymous.Anonymous;
        let result = (inner.vt == VT_UI2).then(|| inner.Anonymous.uiVal);
        let _ = PropVariantClear(&mut value);
        Orientation::from_exif(result? as u8)
    }
}
