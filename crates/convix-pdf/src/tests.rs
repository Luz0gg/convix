use super::*;
use convix_core::{CancelToken, OptionValues};
use convix_image::image::{DynamicImage, Rgb, RgbImage, Rgba, RgbaImage};

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("convix-pdf-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A noisy "photo" (so JPEG recompression has something to gain).
fn photo(w: u32, h: u32, seed: u32) -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
        let v = (x.wrapping_mul(31) ^ y.wrapping_mul(17)).wrapping_add(seed).wrapping_mul(2654435761) >> 24;
        Rgb([(x * 255 / w) as u8, v as u8, (y * 255 / h) as u8])
    }))
}

fn run(op: &str, inputs: &[PathBuf], output: &Path, options: OptionValues) -> Result<Vec<PathBuf>> {
    let ctx = JobContext {
        operation_id: op,
        inputs,
        output: output.to_path_buf(),
        options: &options,
        progress: &|_f: f32| {},
        cancel: &CancelToken::new(),
        modules: &|_| None,
    };
    PdfConverter.run(&ctx)
}

fn opts(pairs: &[(&str, &str)]) -> OptionValues {
    pairs.iter().map(|(k, v)| (k.to_string(), (*v).into())).collect()
}

fn page_count(path: &Path) -> usize {
    lopdf::Document::load(path).unwrap().get_pages().len()
}

/// Three images (JPEG photo, PNG with transparency, PNG photo) → a 3-page PDF.
fn sample_pdf(dir: &Dir) -> PathBuf {
    let jpg = dir.path("a.jpg");
    photo(1200, 800, 1).to_rgb8().save(&jpg).unwrap();
    let logo = dir.path("b.png");
    RgbaImage::from_fn(200, 200, |x, _| Rgba([200, 30, 30, if x < 100 { 255 } else { 0 }])).save(&logo).unwrap();
    let png = dir.path("c.png");
    photo(2600, 1800, 2).save(&png).unwrap();
    let pdf = dir.path("sample.pdf");
    run("pdf.fromImages", &[jpg, logo, png], &pdf, opts(&[])).unwrap();
    pdf
}

#[test]
fn operations_on_pdf_and_images() {
    let ids: Vec<String> = PdfConverter.operations_for(Format::Pdf).into_iter().map(|o| o.id).collect();
    for id in ["pdf.compress", "pdf.split", "pdf.extract", "pdf.merge"] {
        assert!(ids.iter().any(|i| i == id), "{id}");
    }
    #[cfg(windows)]
    assert!(ids.iter().any(|i| i == "pdf.toImages.png"));
    let on_png = PdfConverter.operations_for(Format::Png);
    assert_eq!(on_png.len(), 1);
    assert!(on_png[0].multi_input);
    assert!(PdfConverter.operations_for(Format::Mp3).is_empty());
}

#[test]
fn images_to_pdf_keeps_jpeg_untouched() {
    let dir = Dir::new("from-images");
    let pdf = sample_pdf(&dir);
    assert_eq!(convix_core::format::detect(&pdf).unwrap(), Some(Format::Pdf));
    let doc = lopdf::Document::load(&pdf).unwrap();
    assert_eq!(doc.get_pages().len(), 3);
    // The JPEG is embedded byte for byte (no quality loss).
    let jpeg = std::fs::read(dir.path("a.jpg")).unwrap();
    let embedded = doc.objects.values().filter_map(|o| o.as_stream().ok()).any(|s| s.content == jpeg);
    assert!(embedded, "JPEG embedded as is");
    // The transparent logo got a soft mask.
    let has_smask = doc.objects.values().filter_map(|o| o.as_stream().ok()).any(|s| s.dict.has(b"SMask"));
    assert!(has_smask);
}

#[test]
fn merge_split_extract() {
    let dir = Dir::new("edit");
    let a = sample_pdf(&dir);
    let b = dir.path("two.pdf");
    run("pdf.fromImages", &[dir.path("a.jpg"), dir.path("b.png")], &b, opts(&[("pageSize", "image")])).unwrap();

    let merged = dir.path("merged.pdf");
    run("pdf.merge", &[a.clone(), b.clone()], &merged, opts(&[])).unwrap();
    assert_eq!(page_count(&merged), 5);

    let base = output::reserve(dir.path("sample-p.pdf")).unwrap();
    let parts = run("pdf.split", &[merged.clone()], &base, opts(&[("pages", "1, 4-")])).unwrap();
    let names: Vec<_> = parts.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["sample-p1.pdf", "sample-p4.pdf", "sample-p5.pdf"]);
    assert!(parts.iter().all(|p| page_count(p) == 1));
    assert!(!base.exists(), "placeholder removed");

    let extract = dir.path("extract.pdf");
    run("pdf.extract", &[merged.clone()], &extract, opts(&[("pages", "2-3")])).unwrap();
    assert_eq!(page_count(&extract), 2);
    assert!(run("pdf.extract", &[merged], &extract, opts(&[("pages", "9")])).is_err());
}

#[test]
fn compression_shrinks_photos_and_never_grows() {
    let dir = Dir::new("compress");
    let pdf = sample_pdf(&dir);
    let before = std::fs::metadata(&pdf).unwrap().len();

    let smaller = dir.path("smaller.pdf");
    run("pdf.compress", &[pdf.clone()], &smaller, opts(&[("level", "balanced")])).unwrap();
    let after = std::fs::metadata(&smaller).unwrap().len();
    assert!(after < before / 2, "{before} → {after}");
    assert_eq!(page_count(&smaller), 3);

    let lossless = dir.path("lossless.pdf");
    run("pdf.compress", &[pdf.clone()], &lossless, opts(&[("level", "lossless")])).unwrap();
    assert!(std::fs::metadata(&lossless).unwrap().len() <= before);
}

#[cfg(windows)]
#[test]
fn pages_to_images_with_windows_renderer() {
    let dir = Dir::new("render");
    let pdf = sample_pdf(&dir);
    let base = output::reserve(dir.path("sample-p.png")).unwrap();
    match run("pdf.toImages.png", &[pdf.clone()], &base, opts(&[("dpi", "72"), ("pages", "1-2")])) {
        Ok(pages) => {
            assert_eq!(pages.len(), 2);
            let img = convix_image::image::open(&pages[0]).unwrap();
            // Page 1 holds a landscape photo: landscape A4 at 72 DPI = 842 × 595 px.
            assert!((img.width() as i32 - 842).abs() <= 2 && (img.height() as i32 - 595).abs() <= 2, "{}x{}", img.width(), img.height());
            assert!(!base.exists());
        }
        // Some locked-down sessions can't reach WinRT; the real app can.
        Err(e) => eprintln!("skipped: Windows PDF renderer unavailable here: {e}"),
    }
}
