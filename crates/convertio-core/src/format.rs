//! File formats known to ConvertIO and how to detect them.

use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Image,
    Audio,
    Video,
    Document,
}

/// A file format. Serialized as its lowercase name (`"png"`, `"jpeg"`…), which is also
/// what the frontend receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    // Images
    Png,
    Jpeg,
    Webp,
    Avif,
    Gif,
    Bmp,
    Tiff,
    Ico,
    Heic,
    Svg,
    Tga,
    // Audio
    Mp3,
    Wav,
    Flac,
    Aac,
    M4a,
    Ogg,
    Opus,
    // Video
    Mp4,
    Mov,
    Mkv,
    Webm,
    Avi,
    // Documents
    Pdf,
}

impl Format {
    pub const ALL: &'static [Format] = &[
        Format::Png,
        Format::Jpeg,
        Format::Webp,
        Format::Avif,
        Format::Gif,
        Format::Bmp,
        Format::Tiff,
        Format::Ico,
        Format::Heic,
        Format::Svg,
        Format::Tga,
        Format::Mp3,
        Format::Wav,
        Format::Flac,
        Format::Aac,
        Format::M4a,
        Format::Ogg,
        Format::Opus,
        Format::Mp4,
        Format::Mov,
        Format::Mkv,
        Format::Webm,
        Format::Avi,
        Format::Pdf,
    ];

    pub fn kind(self) -> MediaKind {
        use Format::*;
        match self {
            Png | Jpeg | Webp | Avif | Gif | Bmp | Tiff | Ico | Heic | Svg | Tga => MediaKind::Image,
            Mp3 | Wav | Flac | Aac | M4a | Ogg | Opus => MediaKind::Audio,
            Mp4 | Mov | Mkv | Webm | Avi => MediaKind::Video,
            Pdf => MediaKind::Document,
        }
    }

    /// Canonical extension used when writing a file of this format.
    pub fn extension(self) -> &'static str {
        use Format::*;
        match self {
            Png => "png",
            Jpeg => "jpg",
            Webp => "webp",
            Avif => "avif",
            Gif => "gif",
            Bmp => "bmp",
            Tiff => "tiff",
            Ico => "ico",
            Heic => "heic",
            Svg => "svg",
            Tga => "tga",
            Mp3 => "mp3",
            Wav => "wav",
            Flac => "flac",
            Aac => "aac",
            M4a => "m4a",
            Ogg => "ogg",
            Opus => "opus",
            Mp4 => "mp4",
            Mov => "mov",
            Mkv => "mkv",
            Webm => "webm",
            Avi => "avi",
            Pdf => "pdf",
        }
    }

    /// Human label shown in the UI (not translated: format names are universal).
    pub fn label(self) -> &'static str {
        use Format::*;
        match self {
            Png => "PNG",
            Jpeg => "JPEG",
            Webp => "WebP",
            Avif => "AVIF",
            Gif => "GIF",
            Bmp => "BMP",
            Tiff => "TIFF",
            Ico => "ICO",
            Heic => "HEIC",
            Svg => "SVG",
            Tga => "TGA",
            Mp3 => "MP3",
            Wav => "WAV",
            Flac => "FLAC",
            Aac => "AAC",
            M4a => "M4A",
            Ogg => "OGG",
            Opus => "Opus",
            Mp4 => "MP4",
            Mov => "MOV",
            Mkv => "MKV",
            Webm => "WebM",
            Avi => "AVI",
            Pdf => "PDF",
        }
    }

    /// Parse an extension (case-insensitive, aliases accepted).
    pub fn from_extension(ext: &str) -> Option<Format> {
        use Format::*;
        Some(match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "png" | "apng" => Png,
            "jpg" | "jpeg" | "jpe" | "jfif" => Jpeg,
            "webp" => Webp,
            "avif" => Avif,
            "gif" => Gif,
            "bmp" | "dib" => Bmp,
            "tif" | "tiff" => Tiff,
            "ico" => Ico,
            "heic" | "heif" => Heic,
            "svg" => Svg,
            "tga" | "targa" => Tga,
            "mp3" => Mp3,
            "wav" | "wave" => Wav,
            "flac" => Flac,
            "aac" => Aac,
            "m4a" => M4a,
            "ogg" | "oga" => Ogg,
            "opus" => Opus,
            "mp4" | "m4v" => Mp4,
            "mov" | "qt" => Mov,
            "mkv" => Mkv,
            "webm" => Webm,
            "avi" => Avi,
            "pdf" => Pdf,
            _ => return None,
        })
    }
}

/// Detect a file's format: content signature first (a `.png` that is really a JPEG
/// is reported as JPEG), then the extension for formats without a reliable signature (SVG…).
pub fn detect(path: &Path) -> std::io::Result<Option<Format>> {
    let mut head = [0u8; 8192];
    let n = File::open(path)?.read(&mut head)?;
    Ok(detect_bytes(&head[..n]).or_else(|| detect_extension(path)))
}

pub fn detect_bytes(bytes: &[u8]) -> Option<Format> {
    infer::get(bytes).and_then(|t| Format::from_extension(t.extension()))
}

pub fn detect_extension(path: &Path) -> Option<Format> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(Format::from_extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_aliases() {
        assert_eq!(Format::from_extension("JPG"), Some(Format::Jpeg));
        assert_eq!(Format::from_extension(".jfif"), Some(Format::Jpeg));
        assert_eq!(Format::from_extension("tif"), Some(Format::Tiff));
        assert_eq!(Format::from_extension("xyz"), None);
    }

    #[test]
    fn signature_beats_extension() {
        let png_magic = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
        assert_eq!(detect_bytes(&png_magic), Some(Format::Png));
    }

    #[test]
    fn every_format_roundtrips_through_its_extension() {
        for &f in Format::ALL {
            assert_eq!(Format::from_extension(f.extension()), Some(f), "{f:?}");
        }
    }
}
