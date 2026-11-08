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
    let parts = run("pdf.split", std::slice::from_ref(&merged), &base, opts(&[("pages", "1, 4-")])).unwrap();
    let names: Vec<_> = parts.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["sample-p1.pdf", "sample-p4.pdf", "sample-p5.pdf"]);
    assert!(parts.iter().all(|p| page_count(p) == 1));
    assert!(!base.exists(), "placeholder removed");

    let extract = dir.path("extract.pdf");
    run("pdf.extract", std::slice::from_ref(&merged), &extract, opts(&[("pages", "2-3")])).unwrap();
    assert_eq!(page_count(&extract), 2);
    assert!(run("pdf.extract", &[merged], &extract, opts(&[("pages", "9")])).is_err());
}

#[test]
fn compression_shrinks_photos_and_never_grows() {
    let dir = Dir::new("compress");
    let pdf = sample_pdf(&dir);
    let before = std::fs::metadata(&pdf).unwrap().len();

    let smaller = dir.path("smaller.pdf");
    run("pdf.compress", std::slice::from_ref(&pdf), &smaller, opts(&[("level", "balanced")])).unwrap();
    let after = std::fs::metadata(&smaller).unwrap().len();
    assert!(after < before / 2, "{before} → {after}");
    assert_eq!(page_count(&smaller), 3);

    let lossless = dir.path("lossless.pdf");
    run("pdf.compress", std::slice::from_ref(&pdf), &lossless, opts(&[("level", "lossless")])).unwrap();
    assert!(std::fs::metadata(&lossless).unwrap().len() <= before);
}

#[cfg(windows)]
#[test]
fn pages_to_images_with_windows_renderer() {
    let dir = Dir::new("render");
    let pdf = sample_pdf(&dir);
    let base = output::reserve(dir.path("sample-p.png")).unwrap();
    match run("pdf.toImages.png", std::slice::from_ref(&pdf), &base, opts(&[("dpi", "72"), ("pages", "1-2")])) {
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

/// Media box width of each page, in order: tells pages apart in the sample documents.
fn page_widths(path: &Path) -> Vec<i64> {
    let doc = lopdf::Document::load(path).unwrap();
    doc.get_pages()
        .values()
        .map(|id| {
            let page = doc.get_dictionary(*id).unwrap();
            let mb = page.get(b"MediaBox").unwrap().as_array().unwrap();
            mb[2].as_float().map(|f| f as i64).or_else(|_| mb[2].as_i64()).unwrap()
        })
        .collect()
}

#[test]
fn extract_keeps_the_typed_order() {
    let dir = Dir::new("order");
    // Pages sized like their images: 3 different widths.
    let a = dir.path("a.png");
    let b = dir.path("b.png");
    let c = dir.path("c.png");
    for (p, w) in [(&a, 100), (&b, 200), (&c, 300)] {
        RgbImage::from_pixel(w, 50, Rgb([10, 20, 30])).save(p).unwrap();
    }
    let pdf = dir.path("three.pdf");
    run("pdf.fromImages", &[a, b, c], &pdf, opts(&[("pageSize", "image"), ("margin", "false")])).unwrap();
    let widths = page_widths(&pdf);
    assert_eq!(widths.len(), 3);

    let out = dir.path("reordered.pdf");
    run("pdf.extract", &[pdf], &out, opts(&[("pages", "3, 1")])).unwrap();
    assert_eq!(page_widths(&out), [widths[2], widths[0]]);
}

#[test]
fn split_pages_are_small_and_well_named() {
    let dir = Dir::new("split-size");
    let pdf = sample_pdf(&dir);
    let total = std::fs::metadata(&pdf).unwrap().len();
    // The name is taken: the reserved base becomes "sample-p (1).pdf".
    std::fs::write(dir.path("sample-p.pdf"), b"user file").unwrap();
    let base = output::reserve(dir.path("sample-p.pdf")).unwrap();
    assert!(base.ends_with("sample-p (1).pdf"));
    let parts = run("pdf.split", &[pdf], &base, opts(&[])).unwrap();
    let names: Vec<_> = parts.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["sample-p1.pdf", "sample-p2.pdf", "sample-p3.pdf"]);
    // Each part only carries its own image, not the whole document.
    let sizes: Vec<u64> = parts.iter().map(|p| std::fs::metadata(p).unwrap().len()).collect();
    assert!(sizes.iter().sum::<u64>() < total + total / 5, "{sizes:?} vs {total}");
    assert_eq!(std::fs::read(dir.path("sample-p.pdf")).unwrap(), b"user file");
}

#[test]
fn copy_numbers_are_stripped_only_when_they_are_copy_numbers() {
    assert_eq!(strip_copy_number("doc-p (1)"), "doc-p");
    assert_eq!(strip_copy_number("doc-p (12)"), "doc-p");
    assert_eq!(strip_copy_number("doc (draft)"), "doc (draft)");
    assert_eq!(strip_copy_number("doc ()"), "doc ()");
    assert_eq!(strip_copy_number("doc"), "doc");
}

fn rotations(path: &Path) -> Vec<i64> {
    let doc = lopdf::Document::load(path).unwrap();
    doc.get_pages()
        .values()
        .map(|id| doc.get_dictionary(*id).unwrap().get(b"Rotate").ok().and_then(|o| o.as_i64().ok()).unwrap_or(0))
        .collect()
}

#[test]
fn rotate_and_delete_pages() {
    let dir = Dir::new("rotate");
    let pdf = sample_pdf(&dir); // 3 pages

    let rotated = dir.path("rot.pdf");
    run("pdf.rotate", std::slice::from_ref(&pdf), &rotated, opts(&[("angle", "90"), ("pages", "2-3")])).unwrap();
    assert_eq!(rotations(&rotated), [0, 90, 90]);
    let twice = dir.path("rot2.pdf");
    run("pdf.rotate", &[rotated], &twice, opts(&[("angle", "270")])).unwrap();
    assert_eq!(rotations(&twice), [270, 0, 0], "added to the current rotation");

    let fewer = dir.path("fewer.pdf");
    run("pdf.deletePages", std::slice::from_ref(&pdf), &fewer, opts(&[("pages", "2")])).unwrap();
    assert_eq!(page_count(&fewer), 2);
    assert!(run("pdf.deletePages", std::slice::from_ref(&pdf), &fewer, opts(&[("pages", "1-3")])).is_err(), "never an empty PDF");
    assert!(run("pdf.deletePages", &[pdf], &fewer, opts(&[])).is_err(), "pages are required");
}
