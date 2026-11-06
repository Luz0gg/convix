//! PDF pages → images with the PDF engine built into Windows (`Windows.Data.Pdf`, the one
//! behind the system's PDF previews). Nothing to download, nothing to bundle.

use convix_core::{Error, Result};
use std::path::Path;
use windows::{
    Data::Pdf::{PdfDocument, PdfPageRenderOptions},
    Storage::{
        StorageFile,
        Streams::{DataReader, InMemoryRandomAccessStream},
    },
    Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx},
    core::{HSTRING, Interface},
};
use windows_future::IAsyncOperation;

/// Longest side of a rendered page, in pixels.
const MAX_RENDER_SIDE: f32 = 16000.0;

fn win_err(context: &str) -> impl Fn(windows::core::Error) -> Error + '_ {
    move |e| Error::Failed(format!("{context} ({})", e.message()))
}

/// An open document, ready to render pages.
pub struct Rendered {
    doc: PdfDocument,
}

impl Rendered {
    pub fn open(path: &Path) -> Result<Self> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let absolute = std::path::absolute(path)?;
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(absolute.as_os_str()))
            .and_then(|op| op.join())
            .map_err(win_err("fichier inaccessible"))?;
        let doc = PdfDocument::LoadFromFileAsync(&file)
            .and_then(|op| op.join())
            .map_err(|e| {
                if doc_is_protected(&e) {
                    Error::Failed("ce PDF est protégé par un mot de passe".into())
                } else {
                    Error::Failed(format!("Windows n'arrive pas à lire ce PDF ({})", e.message()))
                }
            })?;
        Ok(Self { doc })
    }

    pub fn page_count(&self) -> Result<u32> {
        self.doc.PageCount().map_err(win_err("PDF"))
    }

    /// Render page `index` (0-based) at `dpi`; returns PNG bytes.
    pub fn page_png(&self, index: u32, dpi: u32) -> Result<Vec<u8>> {
        let page = self.doc.GetPage(index).map_err(win_err("page introuvable"))?;
        // Page size is in device-independent pixels (1/96 inch).
        let size = page.Size().map_err(win_err("PDF"))?;
        // One factor for both sides, so very large pages (posters, banners) shrink to
        // 16000 px on their longest side instead of being squashed.
        let mut scale = dpi as f32 / 96.0;
        let longest = size.Width.max(size.Height) * scale;
        if longest > MAX_RENDER_SIDE {
            scale *= MAX_RENDER_SIDE / longest;
        }
        let width = (size.Width * scale).round().max(1.0) as u32;
        let height = (size.Height * scale).round().max(1.0) as u32;

        let options = PdfPageRenderOptions::new().map_err(win_err("PDF"))?;
        options.SetDestinationWidth(width).map_err(win_err("PDF"))?;
        options.SetDestinationHeight(height).map_err(win_err("PDF"))?;
        let stream = InMemoryRandomAccessStream::new().map_err(win_err("PDF"))?;
        page.RenderWithOptionsToStreamAsync(&stream, &options)
            .and_then(|op| op.join())
            .map_err(win_err("rendu de la page impossible"))?;

        let len = stream.Size().map_err(win_err("PDF"))? as u32;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).map_err(win_err("PDF"))?).map_err(win_err("PDF"))?;
        reader
            .LoadAsync(len)
            .and_then(|op| op.cast::<IAsyncOperation<u32>>())
            .and_then(|op| op.join())
            .map_err(win_err("PDF"))?;
        let mut bytes = vec![0u8; len as usize];
        reader.ReadBytes(&mut bytes).map_err(win_err("PDF"))?;
        Ok(bytes)
    }
}

fn doc_is_protected(e: &windows::core::Error) -> bool {
    // 0x8007052B = ERROR_WRONG_PASSWORD
    e.code().0 as u32 == 0x8007_052B
}
